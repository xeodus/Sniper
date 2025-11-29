use std::env;
use std::sync::Arc;
use dotenv::dotenv;
use futures_util::{pin_mut, StreamExt};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use tokio::{sync::mpsc, time::{interval, sleep, Duration}};
use tracing::{info, warn, error};
use anyhow::Result;
use uuid::Uuid;
use crate::{backtesting::BackTesting, 
    data::{Candles, OrderReq, OrderType, Side, Signal, TradingBot}, 
    db::Database, rest_client::BinanceClient, websocket::WebSocketClient
};

mod db;
mod signal;
mod data;
mod sign;
mod engine;
mod rest_client;
mod websocket;
mod backtesting;
mod position_manager;
mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt().init();

    info!("Starting the bot..");

    let api_key = env::var("API_KEY").expect("API key not found..");
    let secret_key = env::var("SECRET_KEY").expect("secret key not found..");
    let database_url = env::var("DATABASE_URL").expect("Database url not set..");

    let db = Arc::new(Database::new(&database_url).await?);
    let historical_data: Vec<Candles> = db.load_from_db().await?;
    let decimal_ = Decimal::from_i64(10_000).unwrap();

    let mut backtester = BackTesting::new(decimal_);
    let result = backtester.run(historical_data, "ETHUSDT".to_string()); 
    let binance_client = Arc::new(BinanceClient::new(api_key, secret_key, true));

    let (signal_tx, mut signal_rx) = mpsc::channel::<Signal>(100);
    let (order_tx, mut order_rx) = mpsc::channel::<OrderReq>(100);

    result.print_summary();
   
    let bot = Arc::new(
        TradingBot::new(signal_tx.clone(), order_tx, Decimal::new(1000, 0), 
        binance_client.clone(), db.clone())?);
        
    bot.initializer().await?;
    let bot_clone = bot.clone();
    let position = bot_clone.position_manager.get_orders().await?;

    let order_handler = tokio::spawn(async move {
        let decimal = Decimal::from_f64(100.0).unwrap();
        let signal = signal_rx.recv().await.unwrap();

        while let Some(signal) = signal_rx.recv().await {
            info!("Signal: {:?} {} | Confidence {:.2}", signal.action, signal.symbol, signal.confidence * decimal);
        }

        while let Some(order) = order_rx.recv().await {
            info!("Executed order: {:?}", order);
            if let Err(e) = bot_clone.execute_entry_order(signal.clone(), position.position_side, order.order_type).await {
                tracing::error!("Failed to execute order: {}", e);
            }
        }

        match signal.action {
            Side::Buy => {
                let manual_order = OrderReq {
                    symbol: "ETHUSDT".to_string(),
                    id: Uuid::new_v4().to_string(),
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    size: Decimal::new(1, 0),
                    price: signal.price,
                    tp: Some(Decimal::new(3200, 2)),
                    sl: Some(Decimal::new(2900, 2)),
                    manual: true
                };

                if let Err(e) = bot_clone.place_manual_order(manual_order.clone()).await {
                    tracing::error!("Failed to place buy order on  manual mode: {}", e);
                    return binance_client.cancel_orders(&manual_order).await;
                }
            },
            Side::Sell => {
                let manual_order = OrderReq {
                    id: Uuid::new_v4().to_string(),
                    symbol: "ETHUSDT".to_string(),
                    side: Side::Sell,
                    order_type: OrderType::Market,
                    size: Decimal::new(1, 0),
                    price: signal.price,
                    tp: None,
                    sl: None,
                    manual: true
                };

                if let Err(e) = bot_clone.place_manual_order(manual_order.clone()).await {
                    error!("Failed to place sell order on manual mode: {}", e);
                    return binance_client.cancel_orders(&manual_order).await;
                }
            },
            Side::Hold => {
                info!("Unclear signal received holding position till further trend detection...");
            }
        } 

        let mut interval = interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            match binance_client.account_balance().await {
                Ok(balance) => {
                    info!("Account balance: {}", balance);
                },
                Err(e) => {
                    tracing::error!("Failed to get account balance: {}", e);
                }
            }
        }
    }); 

    let symbol = "ETH/USDT";
    let symbol_lower = symbol.to_lowercase().replace("/", "");

    info!("Connecting to the market for symbol: {}", symbol);

    let bot_clone = bot.clone();

    let ws_handler = tokio::spawn(async move {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(30);
        let ws = WebSocketClient::new(&symbol_lower, "1m");

        loop {
            let stream = match ws.connect().await {
                Ok(s) => {
                    info!("WebSocket connected!");
                    backoff = Duration::from_secs(1);
                    s
                },
                Err(e) => {
                    tracing::error!("WebSocket connection failed: {}", e);
                    sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, max_backoff);
                    continue;
                }
            };

            pin_mut!(stream); 

            while let Some(candle_result) = stream.next().await {
                match candle_result {
                    Ok(candle) => {
                        info!("{} | open: {}, high: {}, low: {}, close: {}, volume: {}",
                            symbol, candle.open, candle.high, candle.low, candle.close, candle.volume);

                        if let Err(e) = bot_clone.process_candle(candle, symbol).await {
                            tracing::error!("Failed to process candle data: {}", e);
                            return;
                        }
                    },
                    Err(e) => {
                        tracing::error!("WebSocket connection failed: {}", e);
                        return;
                    }
                }
            }

            warn!("WebSocket stream ended, reconnecting... {:#?}", backoff);
            sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, max_backoff);
        } 
    }); 

    info!("WebSocket running; press Ctrl+C to exit!");

    tokio::select! { 
        result = order_handler => {
            error!("Order handler thread stopped unexpectedly: {:?}", result);
        }
        result = ws_handler => {
            error!("WebSocket handler thread stopped unexpectedly: {:?}", result);
        } 
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received!")
        }
    }

    info!("Shutting down...");

    Ok(())
}
