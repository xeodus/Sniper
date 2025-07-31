use std::env;
use tokio::task;
use tokio::sync::mpsc;

use crate::{data::{Exchange, TopOfBook}, engine::Engine,
    exchange::{binance_auth::Binance, config::{Config, Exchangecfg}, 
    kucoin_auth::KuCoin, StreamBook}, strategy::market_making::MM
};

mod tests;
mod exchange;
mod utils;
mod data;
mod strategy;
mod engine;
mod ws_stream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    let cfg = Config {
        kucoin: Exchangecfg {
            api_key: env::var("API_KEY").expect("KuCoin API key is not set, or not found"),
            secret_key: env::var("SECRET_KEY").expect("KuCoin secret key is not set, or not found")
        },
        binance: Exchangecfg {
            api_key: env::var("API_KEY1").expect("Binance API key is not set, or not found"),
            secret_key: env::var("SECRET_KEY1").expect("Binance secret key is not set, or not found")
        },
        paper: true
    };
    let kucoin_symbol = "ETH-USDT";
    let binance_symbol = "ETHUSDT";
    let (tx, mut rx) = mpsc::unbounded_channel::<TopOfBook>();
    let exchange1 = Exchange::KuCoin;
    let exchange2 = Exchange::Binance;
    let cfg1 = cfg.kucoin.clone();
    let cfg2 = cfg.binance.clone();

    task::spawn(async move {
        if matches!(exchange1, Exchange::KuCoin) {
            let mut ws1 = KuCoin::new(cfg1, kucoin_symbol).await.unwrap();
            while let Ok(tob) = ws1.next_tob().await {
                let _ = tx.send(tob);
            }
        }
        else if matches!(exchange2, Exchange::Binance) {
            let mut ws2 = Binance::new(cfg2, binance_symbol).await.unwrap();
            while let Ok(tob) = ws2.next_tob().await {
                let _ = tx.send(tob);
            }
        }
    });
    
    let client1 = KuCoin::new(cfg.kucoin.clone(), kucoin_symbol).await.unwrap();
    let client2 = Binance::new(cfg.binance.clone(), binance_symbol).await.unwrap();
    let mut engine1 = Engine::new(client1, cfg.paper);
    let mut engine2 = Engine::new(client2, cfg.paper);
    let mut mm = MM::new();

    while let Some(tob_) = rx.recv().await {
        if let Some(order) = mm.decide(&tob_) {
            if matches!(tob_.exchange, Exchange::KuCoin) {
                engine1.handle(&order, &[order.price]).await.unwrap();
            }
            else if matches!(tob_.exchange, Exchange::Binance) {
                engine2.handle(&order, &[order.price]).await.unwrap();
            }   
        }
    }

    Ok(())
}
