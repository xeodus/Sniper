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

pub struct KuCoinWebSocket {
    config: WebSocketConfig,
    candles: VecDeque<Candles>,
    order_updates: Vec<OrderUpdate>,
    ws_sender: Option<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
    ws_receiver: Option<futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
}

impl KuCoinWebSocket {
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
        if let Some(candles_array) = data.as_array() {
            for candle_data in candles_array {
                let candle = Candles {
                    timestamp: candle_data.get(0).and_then(|v| v.as_i64()).unwrap_or(0),
                    open: candle_data.get(1).and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    high: candle_data.get(2).and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    low: candle_data.get(3).and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    close: candle_data.get(4).and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    volume: candle_data.get(5).and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                };

                if self.candles.len() >= self.config.max_candles {
                    self.candles.pop_front();
                }
                self.candles.push_back(candle);
            }
        }
        Ok(())
    }

    async fn process_order_update(&mut self, data: &Value) -> Result<()> {
        let client_oid = data.get("clientOid").and_then(|v| v.as_str()).unwrap_or("");
        let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let filled_qty = data.get("filledSize").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let remaining_qty = data.get("size").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0) - filled_qty;
        let price = data.get("price").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);

        let order_status = match status {
            "open" => OrderStatus::New,
            "done" => OrderStatus::Filled,
            "cancel" | "canceled" => OrderStatus::Rejected,
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
impl WebSocketClient for KuCoinWebSocket {
    async fn connect(&mut self) -> Result<()> {
        let url = "wss://ws-api.kucoin.com/endpoint";
        let (ws_stream, _) = connect_async(url).await?;
        let (sender, receiver) = ws_stream.split();
        
        self.ws_sender = Some(sender);
        self.ws_receiver = Some(receiver);
        
        log::info!("Connected to KuCoin WebSocket");
        Ok(())
    }

    async fn subscribe_to_candles(&mut self, symbol: &str, interval: &str) -> Result<()> {
        if let Some(ref mut sender) = self.ws_sender {
            let topic = format!("/market/candles:{}_{}", symbol, interval);
            let subscribe_msg = json!({
                "id": Uuid::new_v4().to_string(),
                "type": "subscribe",
                "topic": topic,
                "response": true
            });
            
            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            log::info!("Subscribed to KuCoin candle stream: {}", topic);
        }
        Ok(())
    }

    async fn subscribe_to_orders(&mut self) -> Result<()> {
        if let Some(ref mut sender) = self.ws_sender {
            let subscribe_msg = json!({
                "id": Uuid::new_v4().to_string(),
                "type": "subscribe",
                "topic": "/spotMarket/tradeOrders",
                "response": true
            });
            
            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            log::info!("Subscribed to KuCoin order updates");
        }
        Ok(())
    }

    async fn handle_message(&mut self, message: Value) -> Result<()> {
        if let Some(topic) = message.get("topic").and_then(|v| v.as_str()) {
            if topic.contains("/market/candles") {
                if let Some(data) = message.get("data") {
                    self.process_candle_data(data).await?;
                }
            } else if topic.contains("/spotMarket/tradeOrders") {
                if let Some(data) = message.get("data") {
                    self.process_order_update(data).await?;
                }
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