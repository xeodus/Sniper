use chrono::Utc;
use futures_util::{SinkExt, TryStreamExt};
use reqwest::Client;
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::{protocol::frame::coding::Data, Message}};
use std::collections::BTreeMap;

pub struct DataConfig {
    pub rest_base_api: String,
    pub ws_base_api: String,
    pub symbol: String,
    pub depth_levels: usize,
    pub recv_window_ms: Option<u64>
}

#[derive(Debug, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub quantity: f64
}

#[derive(Debug, Deserialize)]
pub struct DepthSnapshot {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub last_updated_id: u64
}

pub struct DepthUpdate {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub updated_id: u64
}

enum MarketEvent {
    Snapshot(DepthSnapshot),
    Update(DepthUpdate),
}

async fn connect_rest_api(base_url: &str, endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(format!("{}/{}", base_url, endpoint)).send().await?;
    let status_code = response.status();

    if !status_code.is_success() {
        return Err(format!("Invaild response received: {}", response.text().await?).into());
    }

    println!("REST response: {}", response.text().await?);
    Ok(())
}

async fn connect_websocket(base_url: &str, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (mut socket, _) = connect_async(format!("{}/{}", base_url, symbol)).await?;
    socket.send(Message::Text(format!("subscribe_depth_updates{}", symbol))).await?;

    while let Ok(Some(msg)) = socket.try_next().await {
        let msg = msg;
        println!("Received: {}", msg);
    }
    Ok(())
}

async fn fetch_snapshot(config: &DataConfig) -> Result<DepthSnapshot, Box<dyn std::error::Error>> {
    let timestamp = Utc::now().timestamp_millis();
    let mut params = BTreeMap::new();
    params.insert("symbol", "BTCUSDT".to_string());
    params.insert("limits", config.depth_levels.to_string());
    params.insert("timestamp", timestamp.to_string());
    let url = "https://binance.com/api/v3/depth";
    let client = Client::new();
    let response = client.get(url).query(&params).send().await?;
    let status_code = response.status();

    if !status_code.is_success() {
        return Err(format!("Invaild snapshot response received: {}", response.text().await?).into());
    }

    let snapshot = response.json().await?;
    Ok(snapshot)
}

async fn fetch_update(config: &DataConfig) -> Result<DepthUpdate, Box<dyn std::error::Error>> {
    let init_update = fetch_snapshot(config).await?;
    let update = DepthUpdate {
        symbol: init_update.symbol,
        bids: init_update.bids,
        asks: init_update.asks,
        updated_id: init_update.last_updated_id
    };
    Ok(update)
}

async fn emit_market_event(event: MarketEvent, config: &DataConfig) -> Result<MarketEvent, Box<dyn std::error::Error>> {
    Ok(match fetch_snapshot(config).await {
        Ok(snapshot) => MarketEvent::Snapshot(snapshot),
        Err(snapshot_err) => {
            match fetch_update(config).await {
                Ok(update) => MarketEvent::Update(update),
                Err(update_err) => return Err(format!("Invaild market event occured: Snapshot Error, {} and Update Error, {}", snapshot_err, update_err).into())
            }
        }
    })
}

