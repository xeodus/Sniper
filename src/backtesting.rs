use csv::Reader;
use serde::Deserialize;
use crate::{execution::Side, market_stream::{DepthSnapshot, DepthUpdate, MarketEvent, OrderBookLevel}, orderbook::{OrderBook, OrderBookManager}, risk_manager::{AccountState, OrderRequest, RiskCheckResult, RiskConfig, RiskManager}, strategy::{MarketData, Signal, StrategyManager, TradeState}};

#[derive(Debug, Deserialize)]
struct CsvLoader {
    symbol: String,
    bids: Vec<OrderBookLevel>,
    asks: Vec<OrderBookLevel>,
    update_id: u64
}

fn load_csv(path: &str, event: &MarketEvent) -> Result<Vec<MarketEvent>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_path(path)?;
    let mut data = Vec::new();
    for line in reader.deserialize() {
        let csv_:CsvLoader = line.expect("Cannot read the parsed line..");
        let evt = match event {
            MarketEvent::Snapshot(_) => MarketEvent::Snapshot(DepthSnapshot {
                symbol: csv_.symbol,
                bids: csv_.bids,
                asks: csv_.asks,
                last_updated_id: csv_.update_id
            }),
            MarketEvent::Update(_) => MarketEvent::Update(DepthUpdate {
                symbol: csv_.symbol,
                bids: csv_.bids,
                asks: csv_.asks,
                first_updated_id: 0,
                final_update_id: csv_.update_id
            })
        };
        data.push(evt);
    }
    Ok(data)
}

pub fn generate_marketdata(ob: &OrderBook) -> MarketData {
    MarketData {
        price: (ob.best_bid() + ob.best_ask()) / 2.0,
        quantity: 0.0,
        bids: vec![OrderBookLevel {
            price: 0.0,
            quantity: 0.0
        }],
        asks: vec![OrderBookLevel {
            price: 0.0,
            quantity: 0.0
        }]
    }
}

pub fn run_backtest(csv_path: &str, state: &mut AccountState, trade: &TradeState, event: &MarketEvent, risk_cfg: &mut RiskConfig, ob: &mut OrderBook) -> Result<(), Box<dyn std::error::Error>> {
    let historical_data = load_csv(csv_path, event)?;
    let mut equity_curve = Vec::new();
    let mut strategy = trade.initialize_strategy(); 

    for evt in &historical_data {
        match evt {
            MarketEvent::Snapshot(snapshot) => ob.apply_snapshots(snapshot),
            MarketEvent::Update(update) => ob.apply_updates(update)
        }
        let md = generate_marketdata(ob);
        strategy.update_indicators(&md);
        let signal = strategy.generate_signal(&md, strategy.order_book_depth);

        if signal == Signal::HOLD {
            equity_curve.push(state.account_balance + state.current_position * md.price);
            continue;
        }

        let mut order_req = OrderRequest {
            entry_price: trade.entry_price,
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

        let fill_price = match order_req.side {
            Side::BUY => ob.best_bid(),
            Side::SELL => ob.best_ask(),
            Side::HOLD => 0.0
        };

        let pnl = (fill_price - trade.entry_price) * order_req.quantity;
        state.account_balance += pnl;
        state.current_position += match order_req.side {
            Side::BUY => order_req.quantity,
            Side::SELL => -order_req.quantity,
            Side::HOLD => 0.0
        };
        
        equity_curve.push(state.account_balance + state.current_position * fill_price);

        for (i, v) in equity_curve.iter().enumerate() {
            println!("{:6}: {:.6}", i, v);
        }
    }
    Ok(())
}