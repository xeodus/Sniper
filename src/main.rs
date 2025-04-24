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

// Declared structs for storing necessary objects

#[derive(Debug, Deserialize)]
struct MarketData {
    symbol: String,
    price: f64,
    bids: Vec<(f64,f64)>,
    ask: Vec<(f64, f64)>,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct OrderResponse {
    order_id: i32,
    status: String,
}

#[derive(Debug)]
struct RiskParams {
    window_size: usize,
    alpha: f64,
    order_quantity: f64,
}

// Signature generation by the client

fn generate_signature(secret_key: &str, data: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size.");
    mac.update(data.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    hex::encode(code_bytes)
}

fn build_query_string(symbol: &str, side: &str, quantity: f64) -> String {
    let mut params = BTreeMap::new();
            params.insert("symbol", symbol.to_string());
            params.insert("side", side.to_string());
            params.insert("quantity", quantity.to_string());
            
            // Iterate through the map and format each "key=value" pair
            params.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<String>>().join("&")
}

// Binance client module

struct BinanceData {
    http_client: Client,
    api_key: String,
    secret_key: String,
    base_url: String,
}

trait FetchMarketData {
    async fn get_market_data(&self, symbol: &str) -> Result<MarketData, Box<dyn std::error::Error>>;
    async fn place_order(&self, symbol: &str, side: &str, quantity: f64) -> Result<OrderResponse, Box<dyn std::error::Error>>;
}

impl FetchMarketData for BinanceData {
    async fn get_market_data(&self, symbol: &str) -> Result<MarketData, Box<dyn std::error::Error>> {
        let url = format!("{} {} {}", self.base_url, "/api/v3/ticker/price?symbol=", symbol);
        let response = self.http_client.get(&url).send().await?.error_for_status()?;
        let market_data = response.json().await?;
        Ok(market_data)
    }

    async fn place_order(&self, symbol: &str, side: &str, quantity: f64) -> Result<OrderResponse, Box<dyn std::error::Error>> {
        let query_string = build_query_string(symbol, side, quantity);
        let signature = generate_signature(self.secret_key.as_str(), &query_string);
        let url = format!("{}/api/v3/order?{}&signature={}", self.base_url, query_string, signature);
        let mut header = HashMap::new();
        header.insert("X-MBX-APIKEY", self.api_key.clone());
        let response = self.http_client.post(&url).header("X-MBX-APIKEY", self.api_key.clone()).send().await?;
        let status_code = response.status();

        if !status_code.is_success() {
            return Err(format!("Invaild response received: {}", response.text().await?).into());
        }

        let order_response = response.json().await?;
        Ok(order_response)
    }
}

// Trading model

struct TradeState {
    order_book_depth: usize,
    imbalance_threshold: f64,
    entry_price: f64,
    ema_period: usize,
    ema_value: f64,
    ema_count: usize,
    sma_buffer: VecDeque<f64>, // Initializes the sma by storing market prices
    max_position: f64,
    stop_loss: f64,
}

trait TradingStrategy {
    fn update_ema(&mut self, market: &MarketData) -> f64;
    fn generate_signal(&mut self, market: &MarketData, depth: usize) -> String;
}

impl TradingStrategy for TradeState {

    fn update_ema(&mut self, market: &MarketData) -> f64 {
        let alpha = 2.0 / (self.ema_period as f64 + 1.0);
        
        if self.ema_count < self.ema_period {
            self.sma_buffer.push_back(market.price);
            self.ema_count += 1;
        }
        else if self.ema_count == self.ema_period {
            self.ema_value = self.sma_buffer.iter().sum::<f64>() / self.ema_period as f64 + 1.0;
        }
        else {
            self.ema_value = market.price * alpha + (1.0 - alpha) * self.ema_value;
        }

        self.ema_value
    }

    fn generate_signal(&mut self, market: &MarketData, depth: usize) -> String {
        let bid_pressure: f64 = market.bids.iter().take(depth).map(|(price, quantity)| price * quantity).sum();
        let bid_asks: f64 = market.ask.iter().take(depth).map(|(price, quantity)| price * quantity).sum();
        let imbalance = (bid_pressure - bid_asks) / (bid_pressure + bid_asks);

        match self.max_position {
            0.0 => if imbalance > self.imbalance_threshold && market.price > self.ema_value {
                "BUY".to_string()
            }
            else if imbalance < self.imbalance_threshold && market.price < self.ema_value {
                "SELL".to_string()
            }
            else {
                "HOLD".to_string()
            },

            x if x < 0.0 => if market.price > self.entry_price * (1.0 + self.stop_loss) {
                "BUY".to_string()
            }
            else if imbalance > self.imbalance_threshold || market.price > self.ema_value {
                "BUY".to_string()
            }
            else {
                "HOLD".to_string()
            },

            x if x > 0.0 => if market.price < self.entry_price * (1.0 - self.stop_loss) {
                "SELL".to_string()
            }
            else if imbalance < self.imbalance_threshold || market.price < self.ema_value {
                "SELL".to_string()
            }
            else {
                "HOLD".to_string()
            },

            _=> "HOLD".to_string()
        }
    }
}

// Order execution

async fn execute_order(client: &BinanceData, decision: String, current_position: &mut f64, entry_price: &mut f64, market: &MarketData, risk_params: &RiskParams) -> Result<(), Box<dyn std::error::Error>> {
    let order_side = match decision.as_str() {
        "BUY" => if *current_position == 0.0 {
            "OPEN_LONG"
        }
        else {
            "CLOSE_SHORT"
        },
        "SELL" => if *current_position == 0.0 {
            "OPEN_SHORT"
        }
        else {
            "CLOSE_LONG"
        },
        _=> return Ok(())
    };

    let quantity = match order_side {
        "OPEN_SHORT" | "OPEN_LONG" => risk_params.order_quantity,
        "CLOSE_SHORT" | "CLOSE_LONG" => current_position.abs(),
        _=> 0.0
    };

    let result = client.place_order(&market.symbol.as_str(), order_side, quantity);

    if result.await.is_ok() {
        match order_side {
            "OPEN_LONG" => {
                *current_position += quantity;
                *entry_price = market.price;
            },
            "OPEN_SHORT" => {
                *current_position -= quantity;
                *entry_price = market.price;
            },
            _=> {}
        }
    }
    Ok(())
}

// Error Handling



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