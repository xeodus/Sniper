use async_trait::async_trait;
use crate::data::OrderReq;

pub mod kucoin_auth;
pub mod binance_auth;
pub mod config;

#[async_trait]
pub trait RestClient {
    async fn place_order(&self, req: &OrderReq) -> Result<String, anyhow::Error>;
    async fn cancel_order(&self, req: &OrderReq) -> Result<String, anyhow::Error>;
}
