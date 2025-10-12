use anyhow::Result;
use uuid::Uuid;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use crate::{config::WebSocketCfg, data::{Candles, OrderStatus}, 
    websocket::ws_client::{OrderUpdate, WebSocketClient}};
use async_trait::async_trait;

pub struct KuCoinClient {
    pub config: WebSocketCfg,
    pub candles: Vec<Candles>,
    pub order_updates: Vec<OrderUpdate>,
    pub ws_sender: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    pub ws_receiver: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
}

impl KuCoinClient {
    pub fn new(cfg: WebSocketCfg) -> Self {
        Self {
            config: cfg,
            candles: Vec::new(),
            order_updates: Vec::new(),
            ws_sender: None,
            ws_receiver: None
        }
    }

    async fn process_candle_data(&mut self, data: &Value) -> Result<()> {
        if let Some(candles_array) = data.as_array() {
            for candle in candles_array {
                let candle = Candles {
                    open: candle.get(0).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    high: candle.get(1).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    low: candle.get(2).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    close: candle.get(3).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    volume: candle.get(4).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    timestamp: candle.get(5).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0)
                };

                if self.candles.len() > self.config.max_candles {
                    self.candles.pop();
                }
                self.candles.push(candle);
            }
        }
        Ok(())
    }

    async fn process_order_update(&mut self, data: Value) -> Result<()> {
        let client_oid = data.get("client_oid").and_then(|v| v.as_str()).unwrap().parse()?;
        let status = data.get("status").and_then(|v| v.as_str()).unwrap();
        let filled_size = data.get("filled_size").and_then(|v| v.as_str()).unwrap().parse()?;
        let remaining_size = data.get("remaining_size").and_then(|v| v.as_str()).unwrap().parse()?;
        let price = data.get("price").and_then(|v| v.as_str()).unwrap().parse()?;

        let order_status = match status {
            "new" => OrderStatus::New,
            "filled" => OrderStatus::Filled,
            "rejected" => OrderStatus::Rejected,
            &_ => {
                log::warn!("Invalid status received, marking as rejected!");
                OrderStatus::Rejected
            }
        };

        let order_update = OrderUpdate {
            client_oid,
            status: order_status,
            filled_size,
            remaining_size,
            price
        };

        self.order_updates.push(order_update);
        Ok(())
    }
}

#[async_trait]
impl WebSocketClient for KuCoinClient {
    async fn connect(&mut self) -> Result<()> {
        let url = "wss://ws-api.kucoin.com/endpoint";
        let (ws_stream, _) = connect_async(url).await?; 
        let (sender, receiver) = ws_stream.split();
        self.ws_sender = Some(sender);
        self.ws_receiver = Some(receiver);
        Ok(())
    }

    async fn subscribe_to_candles(&mut self, symbol: &str, interval: &str) -> Result<()> {
        if let Some(ref mut sender) = self.ws_sender {
            let topic = format!("/market/candles:{}_{}", symbol, interval);
            let subscriber_msg = json!({
                "id": Uuid::new_v4().to_string(),
                "type": "subscribe",
                "topic": topic,
                "response": true
            });
            sender.send(Message::Text(subscriber_msg.to_string())).await?;
        }

        log::info!("Subscribed to KuCoin candle stream!");
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
        }

        log::info!("Subscribed to KuCoin order data stream!");
        Ok(())
    }

    async fn handle_messages(&mut self, messages: Value) -> Result<()> {
        if let Some(topic) = messages.get("topic").and_then(|v| v.as_str()) {
            if topic.contains("/market/candles") {
                if let Some(data) = messages.get("data") {
                    self.process_candle_data(data).await?;
                }
            }
            else if topic.contains("/spotMarket/tradeOrders") {
                if let Some(data) = messages.get("data") {
                    self.process_candle_data(data).await?;
                }
            }
        }
        Ok(())
    }

    async fn get_candles(&self) -> &Vec<Candles> {
        &self.candles
    }

    async fn get_orders(&self) -> &Vec<OrderUpdate> {
        &self.order_updates
    }
}
