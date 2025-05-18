use std::{collections::HashMap, env, time::Duration};
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

async fn prepare_signed_order(query_string: &HashMap<&str, String>) -> Result<OrderResponse, Box<dyn std::error::Error>> {
    // Generate signature
    let query_param = serde_urlencoded::to_string(query_string).unwrap();
    dotenv::dotenv().ok();
    let secret_key = env::var("SECRET_KEY").expect("secret key not found!");
    let signature = generate_signature(&secret_key, &query_param);
    // Generate request
    let final_query_string = format!("{}&signature={}", query_param, signature);
    let api_key = env::var("API_KEY").expect("API key not found!");
    let mut headers = HashMap::new();
    headers.insert("X-MBX-APIKEY", &api_key);
    let url = format!("https://api.binance.com/api/v3/order?{}", final_query_string);
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let response = client.post(&url).header("X-MBX-APIKEY", &api_key).send().await?;
    let status_code = response.status();
    
    if !status_code.is_success() {
        return Err(format!("Invaild response received: {}", response.text().await?).into());
    }

    let order_res = response.json::<OrderResponse>().await?;
    Ok(order_res)
}

pub async fn place_order(side: &Side, type_: &OrderType, symbol: &str, price: f64, quantity: f64, rec_window: f64) -> Side {
    let mut attempt = 0;
    let max_attempt = 5;

    if let Side::HOLD = side {
        println!("Cannot place the order, on HOLD");
        return Side::HOLD;
    }

    let mut params = HashMap::new();
    params.insert("symbol", symbol.to_string());
    params.insert("quantity", format!("{:.6}", quantity));

    let side_str = match side {
        Side::BUY => "BUY",
        Side::SELL => "SELL",
        _ => "BUY"
    };
    params.insert("side", format!("{:?}", side_str.to_string()));
    
    if matches!(type_, OrderType::LIMIT) {
        params.insert("price", format!("{:.3}", price));
    }

    let type_str = match type_ {
        OrderType::LIMIT => "LIMIT",
        OrderType::MARKET => "MARKET"
    };

    params.insert("type", format!("{:?}", type_str.to_string()));
    params.insert("recvWindow", format!("{}", rec_window as u64));
    params.insert("timestamp", Utc::now().timestamp_millis().to_string());

    match prepare_signed_order(&params).await {
        Ok(order_res) => match order_res.status.as_str() {
            "ASK_FILLED" => {
                println!("Ask order filled: {}", order_res.order_id);
                Side::BUY
            },
            "BID_FILLED" => {
                println!("Bid order filled: {}", order_res.order_id);
                Side::SELL
            },
            "UNFILLED" => {
                println!("Order not filled yet: {}", order_res.order_id);
                Side::HOLD
            },
            _ => {
                println!("Unexpected status: {}", order_res.status);
                Side::HOLD
            }
        },
        Err(e) => {
            eprintln!("Error filling the order: {}", e);
            if attempt > max_attempt {
                println!("Maximun attempts exceeded..");
                return Side::HOLD;
            }
            else {
                attempt += 1;
                println!("Attempts made so far: {} out of {}", attempt, max_attempt);
                return Side::HOLD;
            }
        }
    }
}
