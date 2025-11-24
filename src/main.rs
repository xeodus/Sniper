use std::env;
use std::sync::Arc;
use dotenv::dotenv;
use futures_util::{pin_mut, StreamExt};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use tokio::{sync::mpsc, time::{interval, sleep, Duration}};
use tracing::{info, warn};
use anyhow::Result;
use uuid::Uuid;
use crate::{data::{OrderReq, OrderType, Side, Signal, TradingBot}, 
    db::Database, rest_client::BinanceClient, websocket::WebSocketClient};

mod db;
mod signal;
mod data;
mod sign;
mod engine;
mod rest_client;
mod websocket;
mod position_manager;
mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    info!("Starting the bot..");
   
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("Database url not set..");
    let db = Arc::new(Database::new(&database_url).await?);

    let api_key = env::var("API_KEY").expect("API key not found..");
    let secret_key = env::var("SECRET_KEY").expect("secret key not found..");
    let binance_client = Arc::new(BinanceClient::new(api_key, secret_key, true));
    let (signal_tx, mut signal_rx) = mpsc::channel::<Signal>(100);
    let (order_tx, mut order_rx) = mpsc::channel::<OrderReq>(100);
    
    let bot = Arc::new(
        TradingBot::new(signal_tx, order_tx, Decimal::new(1000, 0), 
        binance_client.clone(), db.clone())?);
        
    bot.initializer().await?;

    tokio::spawn(async move {
        let decimal = Decimal::from_f64(100.0).unwrap();
        while let Some(signal) = signal_rx.recv().await {
            info!("Signal: {:?} {} | Confidence {:.2}", signal.action, signal.symbol, signal.confidence * decimal);
        }
    });

    let bot_clone = bot.clone();

    tokio::spawn(async move {
        while let Some(order) = order_rx.recv().await {
            info!("Executed order: {:?}", order);
            if let Err(e) = bot_clone.execute_order(order).await {
                tracing::error!("Failed to execute order: {}", e);
            }
        }
    });

    let symbol = "ETH/USDT";
    info!("Connecting to the market for symbol: {}", symbol);
    let bot_clone = bot.clone();

    tokio::spawn(async move {
        let ws = WebSocketClient::new(symbol, "1m");
        let stream = match ws.connect().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("WebSocket connection failed: {}", e);
                return;
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

        warn!("WebSocket stream ended, reconnecting...");
    });

    let bot_clone = bot.clone();

    tokio::spawn(async move {
        sleep(Duration::from_secs(30)).await;

        let manual_order = OrderReq {
            symbol: "ETH/USDT".to_string(),
            id: Uuid::new_v4().to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            size: Decimal::new(1, 0),
            price: Decimal::new(1000, 0),
            sl: Some(Decimal::new(2900, 0)),
            tp: Some(Decimal::new(3200, 0)),
            manual: true
        };

        info!("Placing manual orders!");

        if let Err(e) = bot_clone.place_manual_order(manual_order.clone()).await {
            tracing::error!("Failed to place manual order: {}", e);
            return binance_client.cancel_orders(&manual_order).await;
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

    Ok(())
}
