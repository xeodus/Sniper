use std::collections::VecDeque;

use csv::Reader;
use serde::Deserialize;
use crate::{execution::Side, risk_manager::{AccountState, OrderRequest, RiskConfig}, strategy::{MarketData, Signal, StrategyManager, TradeState}};

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
        Side::SELL => base
    }
}

fn run_backtest(csv_path: &str, state: &AccountState, trade: &TradeState, risk_cfg: &RiskConfig) -> Result<(), Box<dyn std::error::Error>> {
    let historical_data = load_csv(csv_path)?;
    let mut equity_curve = Vec::new();
    let mut strategy = TradeState {
        order_book_depth: 10,
        imbalance_threshold: 0.20,
        entry_price: state.price,
        ema_period: 20,
        ema_value: state.price,
        ema_count: 0,
        price_buffer: VecDeque::with_capacity(20),
        max_position: 100.0,
        stop_loss: 0.01
    };

    for md in historical_data {
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
            side: Side::HOLD
        };

    }
    
    Ok(())
}