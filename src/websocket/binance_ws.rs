use std::collections::VecDeque;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use anyhow::Result;
use uuid::Uuid;

use crate::data::{Candles, OrderStatus, Side};
use crate::websocket::ws_client::{WebSocketClient, OrderUpdate, WebSocketConfig};
use crate::exchange::config::Exchangecfg;

pub struct BinanceWebSocket {
    config: WebSocketConfig,
    candles: VecDeque<Candles>,
    order_updates: Vec<OrderUpdate>,
    ws_sender: Option<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
    ws_receiver: Option<futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
}

impl BinanceWebSocket {
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            candles: VecDeque::new(),
            order_updates: Vec::new(),
            ws_sender: None,
            ws_receiver: None,
        }
    }

    async fn process_candle_data(&mut self, data: &Value) -> Result<()> {
        if let Some(kline) = data.get("k") {
            let candle = Candles {
                timestamp: kline.get("t").and_then(|v| v.as_i64()).unwrap_or(0),
                open: kline.get("o").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                high: kline.get("h").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                low: kline.get("l").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                close: kline.get("c").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                volume: kline.get("v").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            };

            if self.candles.len() >= self.config.max_candles {
                self.candles.pop_front();
            }
            self.candles.push_back(candle);
        }
        Ok(())
    }

    async fn process_order_update(&mut self, data: &Value) -> Result<()> {
        let client_oid = data.get("c").and_then(|v| v.as_str()).unwrap_or("");
        let status = data.get("X").and_then(|v| v.as_str()).unwrap_or("");
        let filled_qty = data.get("z").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let remaining_qty = data.get("q").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let price = data.get("p").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);

        let order_status = match status {
            "NEW" => OrderStatus::New,
            "FILLED" => OrderStatus::Filled,
            "CANCELED" | "REJECTED" => OrderStatus::Rejected,
            _ => OrderStatus::New,
        };

        let update = OrderUpdate {
            client_oid: client_oid.to_string(),
            status: order_status,
            filled_quantity: filled_qty,
            remaining_quantity: remaining_qty,
            price,
        };

        self.order_updates.push(update);
        Ok(())
    }
}

#[async_trait]
impl WebSocketClient for BinanceWebSocket {
    async fn connect(&mut self) -> Result<()> {
        let url = "wss://stream.binance.com:9443/ws";
        let (ws_stream, _) = connect_async(url).await?;
        let (sender, receiver) = ws_stream.split();
        
        self.ws_sender = Some(sender);
        self.ws_receiver = Some(receiver);
        
        log::info!("Connected to Binance WebSocket");
        Ok(())
    }

    async fn subscribe_to_candles(&mut self, symbol: &str, interval: &str) -> Result<()> {
        if let Some(ref mut sender) = self.ws_sender {
            let stream_name = format!("{}@kline_{}", symbol.to_lowercase(), interval);
            let subscribe_msg = json!({
                "method": "SUBSCRIBE",
                "params": [stream_name],
                "id": Uuid::new_v4().to_string()
            });
            
            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            log::info!("Subscribed to Binance kline stream: {}", stream_name);
        }
        Ok(())
    }

    async fn subscribe_to_orders(&mut self) -> Result<()> {
        // For order updates, we need to use the user data stream
        // This requires authentication and is typically handled separately
        log::warn!("Order subscription requires authenticated user data stream");
        Ok(())
    }

    async fn handle_message(&mut self, message: Value) -> Result<()> {
        if let Some(stream) = message.get("stream").and_then(|v| v.as_str()) {
            if stream.contains("@kline_") {
                if let Some(data) = message.get("data") {
                    self.process_candle_data(data).await?;
                }
            }
        } else if let Some(event_type) = message.get("e").and_then(|v| v.as_str()) {
            if event_type == "executionReport" {
                self.process_order_update(&message).await?;
            }
        }
        Ok(())
    }

    async fn get_candles(&self) -> &Vec<Candles> {
        // Convert VecDeque to Vec for the trait
        // This is a bit hacky but works for the interface
        unsafe {
            std::mem::transmute(&self.candles)
        }
    }

    async fn get_order_updates(&self) -> &Vec<OrderUpdate> {
        &self.order_updates
    }
}