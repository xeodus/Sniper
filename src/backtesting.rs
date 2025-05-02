use csv::Reader;
use serde::Deserialize;
use crate::{execution::Side, risk_manager::{AccountState, OrderRequest, RiskCheckResult, RiskConfig, RiskManager}, strategy::{MarketData, Signal, StrategyManager, TradeState}};

#[derive(Debug, Deserialize)]
struct CsvLoader {
    price: f64,
    quantity: f64,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>
}

fn load_csv(path: &str) -> Result<Vec<MarketData>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_path(path)?;
    let mut data = Vec::new();
    for line in reader.deserialize() {
        let csv_:CsvLoader = line.expect("Cannot read the parsed line..");
        let md = MarketData {
            price: csv_.price,
            quantity: csv_.quantity,
            bids: csv_.bids,
            asks: csv_.asks
        };
        data.push(md);
    } 
    Ok(data)
}

fn simulate_fill(req: &OrderRequest, md: &MarketData) -> f64 {
    let base = md.price * req.quantity;

    match req.side {
        Side::BUY => -base,
        Side::SELL => base,
        Side::HOLD => 0.0
    }
}

pub fn run_backtest(csv_path: &str, state: &mut AccountState, trade: &TradeState, risk_cfg: &mut RiskConfig) -> Result<(), Box<dyn std::error::Error>> {
    let historical_data = load_csv(csv_path)?;
    let mut equity_curve = Vec::new();
    let mut strategy = trade.initialize_strategy();

    for md in &historical_data {
        strategy.update_indicators(&md);
        let signal = strategy.generate_signal(&md, strategy.order_book_depth);

        if signal == Signal::HOLD {
            equity_curve.push(state.account_balance + state.current_position * md.price);
            continue;
        }

        let mut order_req = OrderRequest {
            entry_price: state.price,
            quantity: md.quantity,
            stop_loss: strategy.stop_loss,
            side: match signal {
                Signal::BUY => Side::BUY,
                Signal::SELL => Side::SELL,
                Signal::HOLD => Side::HOLD
            }
        };

        let checks = [
            risk_cfg.check_balance(&state, &mut order_req),
            risk_cfg.check_quantity(&mut order_req),
            risk_cfg.check_position(&state, &mut order_req),
            risk_cfg.check_drawdown(&state, &mut order_req)
        ];

        if checks.iter().any(|r| r == &RiskCheckResult::REJECTED) {
            equity_curve.push(state.account_balance + state.current_position * md.price);
            continue;
        }

        let pnl = simulate_fill(&order_req, &md);
        state.account_balance += pnl;
        state.current_position += match order_req.side {
            Side::BUY => order_req.quantity,
            Side::SELL => -order_req.quantity,
            Side::HOLD => 0.0
        };
        state.unrealised_pnl = ((1.0 / strategy.entry_price) - (1.0 / state.last_price)) * state.current_position * 0.001;
        equity_curve.push(state.unrealised_pnl + state.account_balance);
        let last_price = historical_data.last().map_or(0.0, |md| md.price);
        let final_equity = state.account_balance + state.current_position * last_price;
        println!("Current Balance: {:.6}", state.account_balance);
        println!("Current Position: {:.3}", state.current_position);
        println!("Final Equity: {:.3}", final_equity);

        for (i, v) in equity_curve.iter().enumerate() {
            println!("{:6}: {:.6}", i, v);
        }
    }
    
    Ok(())
}