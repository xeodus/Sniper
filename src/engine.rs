// TRADE BOT ENGINE

use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};
use reqwest::Client;
use serde_json::Value;
use tokio::sync::broadcast;
use crate::data::MACStrategy;

use crate::{auth::KucoinFuturesAPI, 
            data::{CandleSticks, Config,
            DataManager, KlineQuery, KuCoinGateway, 
            PositionSizer, TradingEngine}, 
            execution::TradingStrategy, 
            ws_stream::MarketData,
        };

impl TradingEngine {
    pub fn new(config: Config,
        strategy: MACStrategy,
        gateway: KuCoinGateway,
        account_balance: f64,
        risk_per_trade: f64,
        data_path: &str,
        market_data_rx: broadcast::Receiver<MarketData>
    ) -> Self
    {
        Self {
            config,
            strategy,
            gateway,
            position_size: PositionSizer::init(account_balance, risk_per_trade),
            data_manager: DataManager::new(data_path),
            client: Client::new(),
            active_position: HashMap::new(),
            market_data_rx
        }
    }

    pub fn get_timestamp() -> String {
        SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string()
    }

    pub async fn get_klines(&self, query: &KlineQuery) -> Result<Vec<CandleSticks>, anyhow::Error> {
        let timestamp = Self::get_timestamp();
        let path = format!(
            "/api/v3/kline/query?symbol={}&granularity={}&from={}&to={}",
            query.symbol, query.interval, query.from_time, query.to_time
        );
        let headers = self.config.header_assembly("GET", &path, &timestamp).await;
        let url = format!("{}/{}", self.config.base_url, path);
        let response = self.client.get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(format!("Invalid response received: {}", response.text().await?)).into());
        }

        let json: Value = response.json().await?;
        let mut candles = Vec::new();

        if let Some(data) = json["data"].as_array() {
            for item in data {
                if let Some(arr) = item.as_array() {
                    if arr.len() >= 6 {
                        let candle = CandleSticks {
                            symbol: query.symbol.clone(),
                            timestamp: arr[0].as_str().unwrap_or("0").parse()?,
                            open: arr[1].as_str().unwrap_or("0").parse()?,
                            high: arr[2].as_str().unwrap_or("0").parse()?,
                            low: arr[3].as_str().unwrap_or("0").parse()?,
                            close: arr[4].as_str().unwrap_or("0").parse()?,
                            volume: arr[5].as_str().unwrap_or("0").parse()?
                        };
                        candles.push(candle);
                    }
                }
            }
        }

        Ok(candles)
    }

    pub async fn run_strategy(&mut self, symbol: &str, interval: &str) -> Result<(), anyhow::Error> {
        println!("Starting the bot for symbol {} with interval {}", symbol, interval);

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let one_hour_ago = timestamp - 3600;

        let query = KlineQuery {
            symbol: symbol.to_string(),
            from_time: one_hour_ago,
            to_time: timestamp,
            limit: Some(100),
            interval: interval.to_string()
        };

        let candles = self.get_klines(&query).await
            .map_err(|e| anyhow::anyhow!(format!("Unable to fetch candle sticks for historical data: {}", e)))?;

        self.strategy.analyze_market(&candles);

        if self.strategy.should_enter_long() {
            println!("Long signal received for symbol {}", symbol);
        }

        if self.strategy.should_enter_short() {
            println!("Short signal received for symbol {}", symbol);
        }

        if let Some(position) = self.active_position.get(symbol) {
            if self.strategy.should_exit_position(position) {
                println!("Exit signal detected for symbol {}", symbol);
            }
        }

        if let Err(e) = self.data_manager.save_to_csv(symbol, timestamp, &candles) {
            eprintln!("Failed to save data to csv: {}", e);
        }

        Ok(())
    }
}