use async_trait::async_trait;
use crate::data::{OrderReq, TopOfBook};

pub mod kucoin_auth;
pub mod binance_auth;
pub mod config;

#[async_trait]
pub trait StreamBook {
    async fn next_tob(&mut self) -> anyhow::Result<TopOfBook>;
}

#[async_trait]
pub trait RestClient {
    async fn place_order(&self, req: &OrderReq) -> anyhow::Result<()>;
    async fn cancel_order(&self, id: &str) -> anyhow::Result<()>;
}