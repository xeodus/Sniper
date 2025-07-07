use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct MarketData {
    pub symbol: String,
    pub current_price: f64,
    pub position: f64,
    pub timestamp: i64,
    pub volume: f64,    
}
pub struct WebSocketBuilder {
    pub url: String,
    pub market_data_tx: broadcast::Sender<MarketData>
}

impl WebSocketBuilder {
    pub fn new(url: String, market_data_tx: broadcast::Sender<MarketData>) -> Self {
        Self {
            url,
            market_data_tx
        }
    }

    pub async fn ws_connect(&self, symbols: &[String]) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (mut write, mut read) = ws_stream.split();
        let subscription_message = self.create_subscribe_message(symbols)?;
        write.send(Message::Text(subscription_message)).await?;
        
        info!("WebSocket connected and subscribed for {} number of symbols ", symbols.len());

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(market_data) = self.parse_marketdata(&text) {
                        let _ = self.market_data_tx.send(market_data);
                    }
                },
                Ok(Message::Close(_)) => {
                    warn!("WebSocket connection closed");
                    break;
                },
                Err(e) => {
                    eprintln!("WebSocket error occured: {}", e);
                    break;
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub fn create_subscribe_message(&self, symbols: &[String]) -> Result<String> {
        let subscription_msg = serde_json::json!({
            "id": "1",
            "type": "subscribe",
            "topic": "/market/ticker",
            "symbols": symbols,
            "response": true
        });
        Ok(subscription_msg.to_string())
    }
    pub fn parse_marketdata(&self, data: &str) -> Result<MarketData> {
        let json: Value = serde_json::from_str(data)?;

        Ok(MarketData {
            symbol: json["data"]["symbol"].as_str().unwrap_or("").parse()?,
            current_price: json["data"]["price"].as_str().unwrap_or("0.0").parse()?,
            position: json["data"]["position"].as_str().unwrap_or("0.0").parse()?,
            timestamp: json["data"]["timestamp"].as_str().unwrap_or("0").parse()?,
            volume: json["data"]["volume"].as_str().unwrap_or("0.0").parse()?
        })
    }
}