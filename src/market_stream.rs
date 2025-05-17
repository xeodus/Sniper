use std::time::Duration;

use futures_util::{stream::BoxStream, SinkExt, StreamExt, TryStreamExt};
use reqwest::Client;
use serde::Deserialize;
use tokio::{sync::broadcast, time::sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_stream::wrappers::BroadcastStream;

pub struct DataConfig {
    pub api_key: String,
    pub secret_key: String,
    pub rest_url: String,
    pub ws_url: String,
    pub symbol: String,
    pub depth_levels: usize
}

#[derive(Debug, Deserialize, Clone)]
pub struct DepthSnapshot {
    pub symbol: String,
    pub bids: Vec<[f64; 2]>,
    pub asks: Vec<[f64; 2]>,
    pub last_updated_id: u64
}

#[derive(Debug, Deserialize, Clone)]
pub struct DepthUpdate {
    pub symbol: String,
    pub bids: Vec<[f64; 2]>,
    pub asks: Vec<[f64; 2]>,
    pub first_updated_id: u64,
    pub final_update_id: u64
}

#[derive(serde::Deserialize)]
pub struct WsDepthEvent {
    pub symbol: String,
    pub first_update_id: u64,
    pub final_update_id: u64,
    pub bids: Vec<[f64; 2]>,
    pub asks: Vec<[f64; 2]>
}

#[derive(Debug, Clone)]
pub enum MarketEvent {
    Snapshot(DepthSnapshot),
    Update(DepthUpdate)
}

pub trait MarketStream {
    fn stream(&self) -> BoxStream<'static, Result<MarketEvent, Box<dyn std::error::Error + Send + Sync>>>;
}

impl MarketStream for DataConfig {

    fn stream(&self) -> BoxStream<'static, Result<MarketEvent, Box<dyn std::error::Error + Send + Sync>>> {
        let rest_url = self.rest_url.clone();
        let ws_url = self.ws_url.clone();
        let symbol = self.symbol.clone();
        let level = self.depth_levels.clone();
        let (tx, rx) = broadcast::channel::<MarketEvent>(20);

        tokio::spawn(async move {
            let mut retry_interval = Duration::from_secs(20);
            let max_retry_interval = 5;
            let mut attempt = 0;
            loop {

                // Fetch initial snapshot
                let snap: DepthSnapshot = match Client::new().get(format!("{}/api/v3/depth?symbol={}&limit={}", rest_url, symbol, level))
                .send()
                .await {
                    Ok(signal) => match signal.json().await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("Cannot fetch json resposne: {}", e);
                            sleep(retry_interval).await;
                            break;
                        }
                    },
                    Err(e) => {
                        eprintln!("Cannot get a snapshot, error in response: {}", e);
                        sleep(retry_interval).await;
                        break;
                    }
                };
                let mut last_updated_id = snap.last_updated_id;
                let _ = tx.send(MarketEvent::Snapshot(snap.clone()));

                // Connect to web socket for incremental updates
                let end_point = format!("{}{}@depth@100ms", ws_url, symbol.to_lowercase());
                // Connection
                let (ws_stream, _) = match connect_async(&end_point).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        eprintln!("Ws connection error: {}", e);
                        break;
                    }
                };
                let (mut write, mut read) = ws_stream.split();
                // Subscribe
                let subs = serde_json::json!({"method":"SUBSCRIBE", "params":[format!("{}@depth@100ms", symbol.to_lowercase())], "id":1});
                let _ = write.send(Message::Text(subs.to_string())).await;
                // Enter the loop
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(txt)) => {
                            // parse JSON to struct
                            if let Ok(evt) = serde_json::from_str::<WsDepthEvent>(&txt) {
                                if evt.first_update_id <= last_updated_id {
                                    continue;
                                }
                                if last_updated_id + 1 <= evt.final_update_id {
                                    last_updated_id = evt.final_update_id;
                                    let update = DepthUpdate {
                                        symbol: evt.symbol.clone(),
                                        bids: Vec::with_capacity(100),
                                        asks: Vec::with_capacity(100),
                                        first_updated_id: evt.first_update_id,
                                        final_update_id: evt.final_update_id
                                    };
                                    
                                    if evt.first_update_id > last_updated_id + 1 {
                                        break;
                                    }

                                    let _ = tx.send(MarketEvent::Update(update));
                                }
                            }
                        },
                        Ok(Message::Ping(_)) => {
                            let _ = write.send(Message::Pong(vec![])).await;
                        },
                        Ok(_) => {},
                        Err(e) => eprintln!("WS Error: {}", e)
                    }
                }
                attempt += 1;
                if attempt > max_retry_interval {
                    eprintln!("Connection attempt exceeded the maximum limit");
                    break;
                }
                sleep(retry_interval).await;
                retry_interval = (retry_interval * 2).min(Duration::from_secs(20));
            }
        });

        BroadcastStream::new(rx).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>).boxed()
    }
}