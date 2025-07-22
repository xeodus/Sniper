
// TRADING STRATEGY

use std::collections::HashMap;
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

use crate::{auth::KucoinFuturesAPI, data::{CandleSticks, Config, KuCoinGateway, MACStrategy, Order, OrderPosition, Position}, 
    strategy::TechnicalIndicators
    };

pub trait TradingStrategy {
    fn new(fast_period: usize, slow_period: usize, rsi_period: usize) -> Self;
    fn analyze_market(&mut self, candles: &[CandleSticks]);
    fn should_enter_long(&self) -> bool;
    fn should_enter_short(&self) -> bool;
    fn get_stop_loss(&self, entry_price: f64, side: &Position) -> f64;
    fn get_take_profit(&self, entry_price: f64, side: &Position) -> f64;
    fn should_exit_position(&self, position: &OrderPosition) -> bool;
    fn indicators_ready(&self) -> bool;
}

impl TradingStrategy for MACStrategy {

    fn new(fast_period: usize, slow_period: usize, rsi_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            rsi_period,
            rsi_overbought: 70.0,
            rsi_oversold: 30.0,
            indicators: HashMap::new()
        }
    }

    fn analyze_market(&mut self, candles: &[CandleSticks]) {
        if candles.len() < self.slow_period + self.rsi_period { return; }

        let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();

        self.indicators = HashMap::new();

        let slow_ma_values = TechnicalIndicators::calculate_ema(&closes, self.slow_period);
        self.indicators.insert("slow_ma".into(), slow_ma_values);

        let fast_ma_values = TechnicalIndicators::calculate_ema(&closes, self.fast_period);
        self.indicators.insert("fast_ma".into(), fast_ma_values);

        let rsi_values = TechnicalIndicators::calculate_rsi(&closes, self.rsi_period);
        self.indicators.insert("rsi".into(), rsi_values);
    }

    fn should_enter_long(&self) -> bool {

        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.indicators.get("slow_ma").unwrap();
        let fast_ma = self.indicators.get("fast_ma").unwrap();
        let rsi = self.indicators.get("rsi").unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 && !rsi.is_empty() {
            let current_golden_cross = fast_ma[fast_ma.len() - 1] > slow_ma[slow_ma.len() - 1];
            let previous_golden_cross = fast_ma[fast_ma.len() - 2] <= slow_ma[slow_ma.len() - 2];
            let rsi_not_overbought = rsi[rsi.len() - 1] < self.rsi_overbought as f64;   
            return current_golden_cross && !previous_golden_cross && rsi_not_overbought;
        }

        false
    }

    fn should_enter_short(&self) -> bool {

        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.indicators.get("slow_ma").unwrap();
        let fast_ma = self.indicators.get("fast_ma").unwrap();
        let rsi = self.indicators.get("rsi").unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 && !rsi.is_empty() {
            let current_golden_cross = fast_ma[fast_ma.len() - 1] < slow_ma[slow_ma.len() - 1];
            let previous_golden_cross = fast_ma[fast_ma.len() - 2] >= slow_ma[slow_ma.len() - 2];
            let rsi_not_overbought = rsi[rsi.len() - 1] > self.rsi_overbought as f64;
            return current_golden_cross && !previous_golden_cross && rsi_not_overbought;
        }

        false
    }

    fn get_stop_loss(&self, entry_price: f64, side: &Position) -> f64 {
        let stop_loss_pct = 0.02;

        match side {
            Position::LONG => {
                return entry_price * (1.0 - stop_loss_pct);
            },
            Position::SHORT => {
                return entry_price * (1.0 + stop_loss_pct);
            }
        }
    }

    fn get_take_profit(&self, entry_price: f64, side: &Position) -> f64 {
        let take_profit_pct = 0.04;

        match side {
            Position::LONG => {
                return entry_price * (1.0 + take_profit_pct);
            },
            Position::SHORT => {
                return entry_price * (1.0 - take_profit_pct);
            }
        }
    }

    fn should_exit_position(&self, positions: &OrderPosition) -> bool {
        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.indicators.get("slow_ma").cloned().unwrap();
        let fast_ma = self.indicators.get("fast_ma").cloned().unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 {

            match positions.position_side {
                Position::LONG => {
                    let current_bearish = fast_ma[fast_ma.len() - 1] < slow_ma[slow_ma.len() - 1];
                    let previous_bullish = fast_ma[fast_ma.len() - 2] >= slow_ma[slow_ma.len() - 2];
                    return current_bearish && previous_bullish;
                },
                Position::SHORT => {
                    let current_bullish = fast_ma[fast_ma.len() - 1] > slow_ma[slow_ma.len() - 1];
                    let previous_bearish = fast_ma[fast_ma.len() - 2] <= slow_ma[slow_ma.len() - 2];
                    return current_bullish && previous_bearish;
                }
            }
        }

        false
    }

    fn indicators_ready(&self) -> bool {
        let required_indicators = ["fast_ma", "slow_ma", "rsi"];
        
        for k in required_indicators {
            if !self.indicators[k].is_empty() {
                return true;
            }
        }
        false
    }
}

pub trait OrderGateway {
    fn new(cfg: Config) -> Self;
    async fn place_buy_order(&self, req: &OrderPosition) -> Result<Order, reqwest::Error>;
    async fn place_sell_order(&self, req: &OrderPosition) -> Result<Order, reqwest::Error>;
    async fn cancel_order(&self, symbo: &str) -> Result<(), anyhow::Error>;
}

impl OrderGateway for KuCoinGateway {
    fn new(cfg: Config) -> Self {
        Self {
            cfg,
            client: Client::new()
        }
    }
    async fn place_buy_order(&self, req: &OrderPosition) -> Result<Order, reqwest::Error> {
        let path = "/api/v3/orders";
        let body = json!({
            "clientOid": Uuid::new_v4().to_string(),
            "symbol": req.symbol.to_string(),
            "type": "MARKET".to_string(),
            "side": "BUY".to_string(),
            "quantity": req.size.to_string()
        });
        let header = self.cfg.header_assembly("POST", &path, &body.to_string()).await;
        let url = format!("{}{}", self.cfg.base_url, path);
        let response = self.client
            .post(url)
            .headers(header)
            .json(&body)
            .send()
            .await?;

        response.json::<Order>().await
    }
    
    async fn place_sell_order(&self, req: &OrderPosition) -> Result<Order, reqwest::Error> {
        let path = "/api/v3/orders";
        let body = json!({
            "clientOid": Uuid::new_v4().to_string(),
            "symbol": req.symbol.to_string(),
            "type": "MARKET".to_string(),
            "side": "SELL".to_string(),
            "quantity": req.size.to_string()
        });

        let header = self.cfg.header_assembly("POST", &path, &body.to_string()).await;
        let url = format!("{}{}", self.cfg.base_url, path);
        let response = self.client
            .post(url)
            .headers(header)
            .json(&body)
            .send()
            .await?;

        response.json::<Order>().await
    }
    
    async fn cancel_order(&self, symbol: &str) -> Result<(), anyhow::Error> {
        let path = "/api/v3/positions";
        let query = format!("?symbol={}", symbol);
        let url = format!("{}{}", self.cfg.base_url, path);
        let header = self.cfg.header_assembly("DELETE", &format!("{}{}", path, query), "").await;
        self.client.delete(&url).headers(header).send().await?;
        Ok(())
    }
}