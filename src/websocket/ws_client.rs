use async_trait::async_trait;
use crate::data::{Candles, GridOrder, OrderStatus, Side};
use serde_json::Value;
use anyhow::Result;

#[async_trait]
pub trait WebSocketClient {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe_to_candles(&mut self, symbol: &str, interval: &str) -> Result<()>;
    async fn subscribe_to_orders(&mut self) -> Result<()>;
    async fn handle_message(&mut self, message: Value) -> Result<()>;
    async fn get_candles(&self) -> &Vec<Candles>;
    async fn get_order_updates(&self) -> &Vec<OrderUpdate>;
}

#[derive(Debug, Clone)]
pub struct OrderUpdate {
    pub client_oid: String,
    pub status: OrderStatus,
    pub filled_quantity: f64,
    pub remaining_quantity: f64,
    pub price: f64,
}

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub symbol: String,
    pub interval: String,
    pub max_candles: usize,
}