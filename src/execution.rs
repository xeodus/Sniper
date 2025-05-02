use std::{env, collections::HashMap};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;

#[derive(Debug)]
pub enum Side {
    BUY,
    SELL,
    HOLD
}

#[derive(Debug)]
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

async fn prepare_signed_order<'a>(query_string: &'a HashMap<&'a str, String>) -> Result<OrderResponse, Box<dyn std::error::Error>> {
    // Generate signature
    let query_param = serde_urlencoded::to_string(query_string).unwrap();
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

pub async fn place_order(side: &Side, type_: &OrderType, symbol: &str, price: f64, quantity: f64, rec_window: f64) -> Side {

    let mut params = HashMap::new();
    params.insert("symbol", symbol.to_string());
    params.insert("quantity", format!("{:.8}", quantity));
    params.insert("side", format!("{:?}", side));
    
    if matches!(type_, OrderType::LIMIT) {
        params.insert("price", format!("{:.8}", price));
    }

    params.insert("type", format!("{:?}", type_));
    params.insert("recWindow", format!("{:.3}", rec_window));
    params.insert("timestamp", Utc::now().timestamp_millis().to_string());

    match prepare_signed_order(&params).await {
        Ok(order_res) => match order_res.status {
            val if val == "ask filled".to_owned() => {
                println!("Ask order filled: {}", order_res.order_id);
                Side::BUY
            },
            val if val == "bid filled".to_owned() => {
                println!("Bid order filled: {}", order_res.order_id);
                Side::SELL
            },
            val if val == "unfilled".to_owned() => {
                println!("Order not filled yet: {}", order_res.order_id);
                Side::HOLD
            },
            _ => {
                eprintln!("Error string cases..");
                Side::HOLD
            }       
        },
        Err(e) => {
            eprintln!("Error filling the order: {}", e);
            Side::HOLD
        }
    }
}
