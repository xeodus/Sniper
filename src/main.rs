use crate::{
    backtesting::BackTesting,
    data::{Candles, OrderReq, Signal, TradingBot},
    db::Database,
    rest_client::BinanceClient,
    websocket::WebSocketClient,
};
use anyhow::Result;
use dotenv::dotenv;
use futures_util::{pin_mut, StreamExt};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::env;
use std::sync::Arc;
use tokio::{
    sync::mpsc,
    time::{interval, sleep, Duration},
};
use tracing::{error, info, warn};

mod backtesting;
mod data;
mod db;
mod engine;
mod notification;
mod position_manager;
mod rest_client;
mod sign;
mod signal;
mod websocket;

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

    let bot = Arc::new(TradingBot::new(
        signal_tx.clone(),
        order_tx,
        Decimal::new(1000, 0),
        binance_client.clone(),
        db.clone(),
    )?);

    bot.initializer().await?;

    info!("Trading bot is initialized!");

    let signal_monitor = tokio::spawn(async move {
        while let Some(signal) = signal_rx.recv().await {
            info!(
                "Signal received: Side: {:?}, symbol: {} @ confidence: {:.2}",
                signal.action,
                signal.symbol,
                signal.confidence * Decimal::new(100, 2)
            );
        }
    });

    let order_monitor = tokio::spawn(async move {
        while let Some(order) = order_rx.recv().await {
            info!(
                "Order received: Side: {:?}, symbol: {} @ price: {}",
                order.side, order.symbol, order.price
            );
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
        let mut interval = interval(Duration::from_secs(15));

        loop {
            let stream = match ws.connect().await {
                Ok(s) => {
                    info!("WebSocket connected!");
                    backoff = Duration::from_secs(1);
                    s
                }
                Err(e) => {
                    tracing::error!("WebSocket connection failed: {}", e);
                    sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, max_backoff);
                    continue;
                }
            };

            interval.tick().await;

            match binance_client.account_balance().await {
                Ok(balance) => {
                    info!("Account balance: {}", balance);
                }
                Err(e) => {
                    error!("Failed to get account balance: {}", e);
                }
            }

            pin_mut!(stream);

            while let Some(candle_result) = stream.next().await {
                match candle_result {
                    Ok(candle) => {
                        info!(
                            "{} | open: {}, high: {}, low: {}, close: {}, volume: {}",
                            symbol,
                            candle.open,
                            candle.high,
                            candle.low,
                            candle.close,
                            candle.volume
                        );

                        if let Err(e) = bot_clone.process_candle(candle, symbol).await {
                            tracing::error!("Failed to process candle data: {}", e);
                            return;
                        }
                    }
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
        result = signal_monitor => {
            error!("Signal monitoring thread stopped unexpectedly: {:?}", result);
        }
        result = order_monitor => {
            error!("Order monitoring thread stopped unexpectedly: {:?}", result);
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
