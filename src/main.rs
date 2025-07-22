use std::{collections::HashMap, time::Duration};

use reqwest::Client;
use tokio::signal;

use crate::{auth::KucoinFuturesAPI, 
    data::{Config, DataManager, KuCoinGateway, 
    MACStrategy, PositionSizer, TradingEngine}, 
    execution::{OrderGateway, TradingStrategy}, 
    ws_stream::{MarketData, WebSocketBuilder}
};
mod tests;
mod auth;
mod buffer;
mod data;
mod data_manager;
mod engine;
mod execution;
mod risk_manager;
mod strategy;
mod ws_stream;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let cfg = Config::new(true).unwrap();

    let (tx, rx) = tokio::sync::broadcast::channel::<MarketData>(128);
    let ws_url = "wss://ws-api-sandbox-futures.kucoin.com/endpoint".into();
    tokio::spawn(async move {
        WebSocketBuilder::new(ws_url, tx.clone()).ws_connect(&["ETHUSDTM".into()]).await
    });

    let mut engine = TradingEngine {
        config: cfg.clone(),
        strategy: MACStrategy::new(12, 26, 14),
        position_size: PositionSizer::init(10000.0, 0.02),
        data_manager: DataManager::new("./data"),
        client: Client::new(),
        gateway: KuCoinGateway::new(cfg),
        active_position: HashMap::new(),
        market_data_rx: rx
    };

    let mut ticker = tokio::time::interval(Duration::from_secs(60));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                engine.run_strategy("ETHUSDTM", "1min").await.expect("Strategy run failed..");
            },
            Ok(md) = engine.market_data_rx.recv() => {
                engine.active_position.entry(md.symbol.clone())
                .and_modify(|f| f.market_price = md.current_price);
            },
            _ = signal::ctrl_c() => break
        }
    }
    Ok(())
}