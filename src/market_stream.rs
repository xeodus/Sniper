use futures_util::{stream::BoxStream, SinkExt, StreamExt, TryFutureExt, TryStreamExt};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_stream::wrappers::BroadcastStream;

pub struct DataConfig {
    pub rest_url: String,
    pub ws_url: String,
    pub symbol: String,
    pub depth_levels: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrderBookLevel {
    pub price: f64,
    pub quantity: f64
}

#[derive(Debug, Deserialize, Clone)]
pub struct DepthSnapshot {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub last_updated_id: u64
}

#[derive(Debug, Deserialize, Clone)]
pub struct DepthUpdate {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub first_updated_id: u64,
    pub final_update_id: u64
}

#[derive(serde::Deserialize)]
pub struct WsDepthEvent {
    pub s: String,
    pub u: u64,
    pub u_: u64,
    pub a: Vec<[f64; 2]>,
    pub b: Vec<[f64; 2]>
}

#[derive(Debug, Clone)]
pub enum MarketEvent {
    Snapshot(DepthSnapshot),
    Update(DepthUpdate),
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
            loop {
                // Fetch initial snapshot
                let snap: DepthSnapshot = match Client::new().get(format!("{}/depth&symbol={}&limit={}", rest_url, symbol, level))
                .send()
                .and_then(|r| r.json())
                .await {
                    Ok(signal) => signal,
                    Err(e) => {
                        eprintln!("Cannot fetch json resposne: {}", e);
                        continue;
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
                        continue;
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
                                if evt.u_ <= last_updated_id {
                                    continue;
                                }
                                if evt.u <= last_updated_id + 1 {
                                    last_updated_id = evt.u_;
                                    let update = DepthUpdate {
                                        symbol: evt.s.clone(),
                                        bids: evt.b.into_iter().map(|x| OrderBookLevel {
                                            price: x[0] as f64,
                                            quantity: x[0] as f64
                                        }).collect(),
                                        asks: evt.a.into_iter().map(|x| OrderBookLevel {
                                            price: x[0] as f64,
                                            quantity: x[0] as f64
                                        }).collect(),
                                        first_updated_id: evt.u_,
                                        final_update_id: evt.u
                                    };
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
            }
        });

        BroadcastStream::new(rx).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>).boxed()
    }
}