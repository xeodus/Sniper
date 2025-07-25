use std::fs;
use tokio::task;
use tokio::sync::mpsc;

use crate::{data::TopOfBook, engine::Engine,
    exchange::{auth::KuCoin, config::{self}, StreamBook},
    strategy::market_making::MM
};
mod tests;
mod exchange;
mod utils;
mod data;
mod strategy;
mod engine;
mod ws_stream;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    let cfg: config::Config = toml::from_str(&fs::read_to_string("config.toml")?).unwrap();
    let symbol = "ETH-USDT";
    let (tx, mut rx) = mpsc::unbounded_channel::<TopOfBook>();
    let cfg_ = cfg.kucoin.clone();

    task::spawn(async move {
        let mut ws = KuCoin::new(cfg_, symbol).await.unwrap();
        while let Ok(tob) = ws.next_tob().await {
            let _ = tx.send(tob);
        }   
    });

    let client = KuCoin::new(cfg.kucoin.clone(), symbol).await.unwrap();
    let mut mm = MM::new();
    let mut engine = Engine::new(client, cfg.paper);

    while let Some(tob) = rx.recv().await {
        if let Some(order) = mm.decide(&tob) {
            engine.handle(&order, &[order.price]).await.unwrap();
        }
    }
    Ok(())
}