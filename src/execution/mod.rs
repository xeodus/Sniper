use std::{collections::{BTreeMap, HashMap}, env};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;
use crate::strategy::Signal;

pub enum Side {
    BUY,
    SELL
}

pub enum OrderType {
    MARKET,
    LIMIT
}

#[derive(Debug, Deserialize)]
pub struct OrderResponse {
    order_id: u64,
    status: String

}
fn generate_signature(secret_key: &str, data: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take keys of any size.");
    mac.update(data.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    hex::encode(code_bytes)
}

async fn prepare_order(side: &Side) -> Result<OrderResponse, Box<dyn std::error::Error>> {

    match side {
        Side::BUY => Signal::BUY,
        Side::SELL => Signal::SELL,
        _ => {
            println!("Waiting for a valid signal..");
            Signal::HOLD
        }
    };

    let timestamp = Utc::now().timestamp_millis();
    let mut query_string = HashMap::new();
    query_string.insert("symbol", "BTCUSDT".to_string());
    query_string.insert("price", "75000.0".to_string());
    query_string.insert("quantity", "1.0".to_string());
    query_string.insert("recWindow", "5000".to_string());
    query_string.insert("timestamp", timestamp.to_string());
    // Generate signature
    let query_param = serde_urlencoded::to_string(&query_string).unwrap();
    let secret_key = env::var("SECRET_KEY").expect("secret key not found!");
    let signature = generate_signature(&secret_key, &query_param);
    // Generate request
    let final_query_string = format!("{}&signature={}", query_param, signature);
    let api_key = env::var("API_KEY").expect("API key not found!");
    let mut headers = HashMap::new();
    headers.insert("X-MBX-APIKEY", &api_key);
    let url = "https://api.binance.com/api/v3/order";
    let client = Client::new();
    let response = client.post(url).header("X-MBX-APIKEY", &api_key).body(final_query_string).send().await?;
    let status_code = response.status();
    
    if !status_code.is_success() {
        return Err(format!("Invaild response received: {}", response.text().await?).into());
    }

    let order_res = response.json::<OrderResponse>().await?;
    Ok(order_res)
}