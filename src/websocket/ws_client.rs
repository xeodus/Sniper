use crate::data::{Candles, OrderStatus};
use anyhow::Result;
use serde_json::Value;
use async_trait::async_trait;

pub struct OrderUpdate {
    pub client_oid: String,
    pub status: OrderStatus,
    pub filled_size: f64,
    pub remaining_size: f64,
    pub price: f64
}

#[async_trait]
pub trait WebSocketClient {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe_to_candles(&mut self, symbol: &str, interval: &str) -> Result<()>;
    async fn subscribe_to_orders(&mut self) -> Result<()>;
    async fn handle_messages(&mut self, messages: Value) -> Result<()>;
    async fn get_candles(&self) -> &Vec<Candles>;
    async fn get_orders(&self) -> &Vec<OrderUpdate>;
}
