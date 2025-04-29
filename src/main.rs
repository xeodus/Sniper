pub mod data_handling;
pub mod strategy;
pub mod execution;
pub mod risk_manager;
pub mod backtesting;
use std::{collections::{BTreeMap, VecDeque}, time::Duration};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::collections::HashMap;
use reqwest::Client;

// Main function

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BinanceData {
        http_client: reqwest::Client::new(),
        api_key: std::env::var("API_KEY").expect("api key not set!"),
        secret_key: std::env::var("SECRET_KEY").expect("secret key not set!"),
        base_url: std::env::var("BASE_URL").expect("base url not found!"),
    };

    let mut state = TradeState {
        order_book_depth: 10,
        imbalance_threshold: 0.4,
        entry_price: 0.0,
        ema_value: 0.0,
        ema_count: 0,
        ema_period: 20,
        max_position: 100.0,
        stop_loss: 0.01,
        sma_buffer: VecDeque::new(),
    };

    let symbol = "BTCUSDT";
    let params = RiskParams {
        window_size: 10,
        alpha: 0.1,
        order_quantity: 1.0,
    };

    loop {
        let market_data = match client.get_market_data(&symbol).await {
            Ok(data) => data,
            Err(_e) => {
                eprintln!("Error fetching the market data!");
                continue;
            }
        };

        state.update_ema(&market_data);
        let signal = state.generate_signal(&market_data, state.order_book_depth);

        if signal != "HOLD".to_string() {
            execute_order(&client, signal, &mut state.max_position, &mut state.entry_price, &market_data, &params).await?;
        }

        tokio::time::sleep(Duration::from_secs(7)).await;
    }
}

// This bot is under developement, some key features are need to be added.