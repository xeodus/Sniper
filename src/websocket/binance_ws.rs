use uuid::Uuid;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use crate::data::{Candles, OrderStatus};
use crate::websocket::ws_client::{OrderUpdate, WebSocketClient};
use crate::config::WebSocketCfg;
use async_trait::async_trait;

pub struct BinanceClient {
    pub config: WebSocketCfg,
    pub candles: Vec<Candles>,
    pub order_updates: Vec<OrderUpdate>,
    pub ws_sender: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    pub ws_receiver: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>
}

impl BinanceClient {
    pub fn new(config: WebSocketCfg) -> Self {
        Self {
            config,
            candles: Vec::new(),
            order_updates: Vec::new(),
            ws_sender: None,
            ws_receiver: None
        }
    }

    async fn process_candles_data(&mut self, data: &Value) -> Result<()> {
        if let Some(candle_array) = data.as_array() {
            for candle in candle_array {
                let candles = Candles {
                    open: candle.get(0).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    high: candle.get(1).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    low: candle.get(2).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    close: candle.get(3).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    volume: candle.get(4).and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0),
                    timestamp: candle.get(5).and_then(|v| v.as_i64()).unwrap_or(0)
                };
                
                if self.candles.len() > self.config.max_candles {
                    self.candles.pop();
                }

                self.candles.push(candles);
            }
        }
        Ok(())
    }

    async fn process_order_update(&mut self, data: &Value) -> Result<()> {
        let client_oid = data.get("client_oid").and_then(|v| v.as_str()).unwrap();
        let status = data.get("status").and_then(|v| v.as_str()).unwrap();
        let filled_size = data.get("filled").and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0);
        let remaining_size = data.get("remaining").and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0);
        let price = data.get("price").and_then(|v| v.as_str()).unwrap().parse().unwrap_or(0.0);

        let order_status = match status {
            "open" => OrderStatus::New,
            "done" => OrderStatus::Filled,
            "cancel" | "canceled" => OrderStatus::Rejected,
            &_ => OrderStatus::New
        };

        let update = OrderUpdate {
            client_oid: client_oid.to_string(),
            status: order_status,
            filled_size,
            remaining_size,
            price
        };

        self.order_updates.push(update);
        Ok(())
    }
}

#[async_trait]
impl WebSocketClient for BinanceClient {
    async fn connect(&mut self) -> Result<()> {
        let url = "wss://stream.binance.com:9443/ws";
        let (ws_stream, _) = connect_async(url).await?;
        let (sender, receiver) = ws_stream.split();
        self.ws_sender = Some(sender);
        self.ws_receiver = Some(receiver);
        Ok(())
    }

    async fn subscribe_to_candles(&mut self, symbol: &str, interval: &str) -> Result<()> {
        if let Some(ref mut sender) = self.ws_sender {
            let stream = format!("{}@kline_{}", symbol.to_lowercase(), interval);
            let subscribe_msg = json!({
                "method": "SUBSCRIBE",
                "params": [stream],
                "id": Uuid::new_v4().to_string()
            });

            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            log::info!("Subscribed to Binance kline stream: {}", stream);
        }
        Ok(())
    }

    async fn subscribe_to_orders(&mut self) -> Result<()> {
        log::warn!("Order subscribe needs authenticated user data stream.");
        Ok(())
    }

    async  fn handle_messages(&mut self, messages: Value) -> Result<()> {
        if let Some(stream) = messages.get("stream").and_then(|v| v.as_str()) {
            if stream.contains("@kline_") {
                if let Some(data) = messages.get("data") {
                    self.process_candles_data(data).await?;
                }
            }
        }
        else if let Some(event_type) = messages.get("e") {
            if event_type == "executionReport" {
                self.process_order_update(&messages).await?;
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
