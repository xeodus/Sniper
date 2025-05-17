use std::collections::VecDeque;
use std::env;
use std::time::Duration;
use futures_util::StreamExt;
use sniper_bot::execution::{place_order, OrderType, Side};
use sniper_bot::market_stream::{DataConfig, MarketEvent, MarketStream};
use sniper_bot::orderbook::{OrderBook, OrderBookManager};
use sniper_bot::risk_manager::{OrderRequest, RiskConfig, RiskManager};
use sniper_bot::strategy::{MarketData, Signal, StrategyManager};
use sniper_bot::{risk_manager::AccountState, strategy::TradeState};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting trading bot..");
    dotenv::dotenv().ok();

    let data_config = DataConfig {
        api_key: env::var("API_KEY").expect("API key is not set!"),
        secret_key: env::var("SECRET_KEY").expect("secret key is not set!"),
        rest_url: "https://binance.com".into(),
        ws_url: "wss://stream.binance.com:9443".into(),
        symbol: "BTCUSDT".into(),
        depth_levels: 15
    };

    println!("Connecting to market stream: {}", data_config.symbol);

    let mut market_stream = data_config.stream();
    let mut ob = OrderBook::initialize();

    let mut trade = TradeState {
        order_book_depth: 10,
        imbalance_threshold: 0.20,
        entry_price: 0.0,
        ema_period: 20,
        ema_value: 0.0,
        ema_count: 0,
        price_buffer: VecDeque::with_capacity(20),
        max_position: 0.001,
        stop_loss: 0.01
    };

    let mut state = AccountState {
        current_position: 0.0,
        max_position: trade.max_position,
        entry_price: 0.0,
        last_price: 0.0,
        account_balance: 0.0,
        unrealised_pnl: 1.24
    };

    let risk_cfg = RiskConfig {
        max_drawdown_pct: 0.20,
        max_position_pct: 0.02,
        warn_position_pct: 0.04,
        max_potential_loss: 0.02
    };

    let req = OrderRequest {
        entry_price: 0.0,
        quantity: 0.0,
        stop_loss: trade.stop_loss,
        side: Side::HOLD
    };

    let md = MarketData {
        price: (ob.best_bid() + ob.best_ask()) / 2.0,
        quantity: 0.0,
        bids: Vec::with_capacity(20),
        asks: Vec::with_capacity(20)
    };

    println!("Waiting for order book data..");

    let mut snapshot_received = false;

    while let Some(result) = market_stream.next().await {
        match result {
            Ok(event) => {
                match event {
                    MarketEvent::Snapshot(snapshot) => {
                        println!("Received order book snapshot for: {} for levels: {}",
                        snapshot.last_updated_id, snapshot.bids.len() + snapshot.asks.len());
                        ob.apply_snapshots(&snapshot);
                        snapshot_received = true;
                        if !snapshot_received {
                            continue;
                        }
                    },
                    MarketEvent::Update(update) => {
                        println!("Received incremental order book update for: {} for levels: {}",
                        update.final_update_id, update.bids.len() + update.asks.len());
                        let updated = ob.apply_updates(&update);
                        if !updated {
                            println!("Didn't receive incremental update, waiting for next snapshot..");
                            continue;
                        }
                    }
                }
                if !snapshot_received || ob.bids.is_empty() || ob.asks.is_empty() {
                    continue;
                }
            },
            Err(e) => {
                eprintln!("Error fetching market event from the market stream: {}", e);
                sleep(Duration::from_secs(3)).await;
                market_stream = data_config.stream();
            }
        }

        let stop_loss_triggered = state.check_stop_loss(md.price, trade.stop_loss);
        if stop_loss_triggered && state.current_position != 0.0 {
            println!("Stop loss triggered at price: {}", md.price);
            let close_side = if state.current_position > 0.0 {
                Side::SELL
            }
            else {
                Side::BUY
            };
            println!("Closing position for side: {:?}, quantity: {:.6} @ {:.6}", close_side, req.quantity, md.price);

            let close_quantity = state.current_position.abs();
            let exe = place_order(
                &req.side,
                &OrderType::MARKET,
                &data_config.symbol,
                md.price,
                md.quantity,
                5000.0
            );

            if let Side::HOLD = exe.await {
                println!("Cannot place order..");
            }
            else {
                let _update_position = state.update_position(close_quantity, md.price);
                println!("Position closed at price: {}", md.price);
            }
            continue;
        }

        let ema = trade.update_indicators(&md);
        let signal = trade.generate_signal(&md, data_config.depth_levels);

        match signal {
            Signal::BUY if  state.current_position <= 0.0 => {
                let quantity = state.calculate_position_size(md.price, risk_cfg.max_position_pct);

                if quantity > 0.0 {
                    println!("Buy signal for price: {}, ema: {} and quantity: {}", md.price, ema, md.quantity);
                }

                let exe = place_order(
                    &req.side,
                    &OrderType::MARKET,
                    &data_config.symbol,
                    md.price,
                    md.quantity,
                    5000.0
                );

                if let Side::BUY = exe.await {
                    state.update_position(quantity, md.price);
                    trade.entry_price = md.price;
                    println!("Buy order executed!");
                }
            },
            Signal::SELL if state.current_position > 0.0 => {
                let quantity = state.calculate_position_size(md.price, risk_cfg.max_position_pct);

                if quantity > 0.0 {
                    println!("Sell signal for price: {}, ema: {} and quantity: {}", md.price, ema, md.quantity);
                }

                let exe = place_order(&req.side,
                    &OrderType::LIMIT,
                    &data_config.symbol,
                    md.price,
                    md.quantity,
                    5000.0
                );

                if let Side::SELL = exe.await {
                    state.update_position(-quantity, md.price);
                    trade.entry_price = md.price;
                    println!("Sell order executed");
                }
            },
            Signal::HOLD => {
                if state.current_position != 0.0 {
                    println!("Order on hold waiting for appropriate signal:
                    - price: {:.2}
                    - ema: {:.2}
                    - position: {:.8}
                    - PnL: {}",
                    md.price,
                    ema,
                    state.current_position,
                    state.unrealised_pnl
                    );
                }
            },
            _ => {}
        }
    }

    tokio::time::sleep(Duration::from_secs(10)).await;
    
    println!("Final account state: ");
    println!("- Position: {}", state.current_position);
    println!("- Balance: {}", state.account_balance);
    println!("- Unrealised PnL: {}", state.unrealised_pnl);
    Ok(())
}
