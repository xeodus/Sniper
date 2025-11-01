use anyhow::{Result,Context};
use futures_util::StreamExt;
use rust_decimal::Decimal;
use tracing::{info, warn};
use crate::data::{BinanceKline, Candles};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct WebSocketClient {
    pub url: String
}

impl WebSocketClient {
    pub fn new(symbol: &str, interval: &str) -> Self {
        let symbol_lower = symbol.to_lowercase().replace("/", "");
        let url = format!("wss://stream.binance.com:9443/ws/{}@kline_{}", symbol_lower, interval);

        Self { url }
    }

    pub async fn connect(&self) -> Result<impl StreamExt<Item = Result<Candles, anyhow::Error>>> {
        let (ws_srteam, _) = connect_async(&self.url).await
            .context("Failed to connect to Binance WebSocket..")?;

        info!("Connected to Binance WebSocket!");

        let (_, read) = ws_srteam.split();
        let stream = read.filter_map(|msg| async move {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<BinanceKline>(&text) {
                        Ok(kline) => {
                            match (
                                kline.open.parse::<f64>(),
                                kline.high.parse::<f64>(),
                                kline.low.parse::<f64>(),
                                kline.close.parse::<f64>(),
                                kline.volume.parse::<f64>()
                            )
                            {
                                (Ok(o), Ok(h), Ok(l), Ok(c), Ok(v)) => {
                                    Some(Ok(Candles {
                                        timestamp: kline.open_time / 1000,
                                        open: Decimal::from_f64_retain(o).unwrap(),
                                        high: Decimal::from_f64_retain(h).unwrap(),
                                        low: Decimal::from_f64_retain(l).unwrap(),
                                        close: Decimal::from_f64_retain(c).unwrap(),
                                        volume: Decimal::from_f64_retain(v).unwrap()
                                    }))
                                },
                                _ => {
                                    warn!("Failed to parse kline data from the WebSocket stream..");
                                    None
                                }
                            }
                        },
                        Err(e) => {
                            warn!("Failed to get kline from the WebSocket: {}", e);
                            None
                        }
                    }
                },
                Ok(Message::Ping(_)) => None,
                Ok(Message::Pong(_)) => None,
                Err(e) => {
                    Some(Err(anyhow::anyhow!("Failed to connect WebSocket: {}", e)))
                },
                _ => None
            }
        });

        Ok(stream)
    }
}
