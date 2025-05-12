use std::io;
use std::collections::VecDeque;
use sniper_bot::backtesting::run_backtest;
use sniper_bot::market_stream::{DepthSnapshot, DepthUpdate, MarketEvent, OrderBookLevel};
use sniper_bot::orderbook::OrderBook;
use sniper_bot::{risk_manager::{AccountState, RiskConfig}, strategy::TradeState};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut trade = TradeState {
        order_book_depth: 10,
        imbalance_threshold: 0.20,
        entry_price: 75000.0,
        ema_period: 20,
        ema_value: 0.0,
        ema_count: 0,
        price_buffer: VecDeque::with_capacity(20),
        max_position: 100.0,
        stop_loss: 0.01
    };

    let mut state = AccountState {
        current_position: 50.0,
        max_position: trade.max_position,
        entry_price: 76000.0,
        last_price: 80000.0,
        account_balance: 50000.0,
        unrealised_pnl: 1.24
    };

    let init_event = MarketEvent::Snapshot(DepthSnapshot {
        symbol: "BTCUSDT".to_string(),
        bids: vec![OrderBookLevel {
            price: 76000.0,
            quantity: 1.0
        }],
        asks: vec![OrderBookLevel {
            price: 76500.0,
            quantity: 1.0
        }],
        last_updated_id: 1001
    });

    let event = match init_event {
        MarketEvent::Snapshot(snapshot) => {
            println!("Got a snapshot for Symbol: {}, Order ID: {}, Bids: {:?}, Asks: {:?}", snapshot.symbol, snapshot.last_updated_id, snapshot.bids, snapshot.asks);
            MarketEvent::Snapshot(DepthSnapshot {
            symbol: "BTCUSDT".into(),
            bids: vec![ OrderBookLevel {
                price: 76000.0,
                quantity: 1.0
            }],
            asks: vec![ OrderBookLevel {
                price: 76500.0,
                quantity: 1.0
            }],
            last_updated_id: 1001
        })},
        MarketEvent::Update(update) => {
            println!("Got incremental update for Symbol: {}, New Order ID: {}, Bids: {:?}, Asks: {:?}", update.symbol, update.final_update_id, update.bids, update.asks);
            MarketEvent::Update(DepthUpdate {
            symbol: "BTCUSDT".into(),
            bids: vec![ OrderBookLevel {
                price: 76200.0,
                quantity: 1.0
            }],
            asks: vec![ OrderBookLevel {
                price: 76250.0,
                quantity: 1.0
            }],
            first_updated_id: 1001,
            final_update_id: 1002
        })}
    };

    let mut risk_cfg = RiskConfig {
        max_drawdown_pct: 0.20,
        max_position_pct: 0.02,
        warn_position_pct: 0.04,
        max_potential_loss: 0.02
    };

    let mut ob = OrderBook {
        bids: vec![OrderBookLevel {
            price: 75000.0,
            quantity: 0.9
        }],
        asks: vec![OrderBookLevel {
            price: 80000.0,
            quantity: 0.9
        }],
        last_update_id: 1001
    };

    match run_backtest("/Users/fever_dreamer/Desktop/Rust/Sniper/data/historical_data.csv", &mut state, &mut trade, &event, &mut risk_cfg, &mut ob) {
        Ok(()) => println!("Backtest ran successfully!"),
        Err(e) => eprintln!("Error occured in backtest: {}", e)
    }
    println!("Final account state: ");
    println!("- Position: {}", state.current_position);
    println!("- Balance: {}", state.account_balance);
    println!("- Unrealised PnL: {}", state.unrealised_pnl);
    Ok(())
}
