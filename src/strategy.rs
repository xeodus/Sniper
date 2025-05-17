use std::collections::VecDeque;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MarketData {
    pub price: f64,
    pub quantity: f64,
    pub bids: Vec<[f64; 2]>,
    pub asks: Vec<[f64; 2]>
}

#[derive(PartialEq)]
pub enum Signal {
    BUY,
    SELL,
    HOLD
}

#[derive(Debug, Deserialize)]
pub struct TradeState {
    pub order_book_depth: usize,
    pub imbalance_threshold: f64,
    pub entry_price: f64,
    pub ema_period: usize,
    pub ema_value: f64,
    pub ema_count: usize,
    pub price_buffer: VecDeque<f64>, // Initializes the sma by storing market prices
    pub max_position: f64,
    pub stop_loss: f64,
}

pub trait StrategyManager {
    fn initialize_strategy(&self) -> Self;
    fn update_indicators(&mut self, market: &MarketData) -> f64;
    fn generate_signal(&mut self, market: &MarketData, depth: usize) -> Signal;
}

impl StrategyManager for TradeState {
    fn initialize_strategy(&self) -> Self {
        Self {
            order_book_depth: self.order_book_depth,
            imbalance_threshold: self.imbalance_threshold,
            entry_price: self.entry_price,
            ema_period: self.ema_period,
            ema_value: self.ema_value,
            ema_count: self.ema_count,
            price_buffer: VecDeque::new(),
            max_position: self.max_position,
            stop_loss: self.stop_loss
        }
    }

    fn update_indicators(&mut self, market: &MarketData) -> f64 {
        let alpha = 2.0 / (self.ema_period as f64 + 1.0);

        if self.ema_count < self.ema_period {
            self.price_buffer.push_back(market.price);
            self.ema_count += 1;

            if self.price_buffer.len() > self.ema_period {
                self.price_buffer.pop_front();
            }

            let sma = self.price_buffer.iter().sum::<f64>() / self.price_buffer.len() as f64;
            self.ema_value = sma;
        }
        else {
            self.ema_value = market.price * alpha + (1.0 - alpha) * self.ema_value;
        }
        self.ema_value
    }

    fn generate_signal(&mut self, market: &MarketData, depth: usize) -> Signal {
        let bid_pressure = market.bids.iter().take(depth).map(|x| x[1]).sum::<f64>();
        let ask_pressure = market.asks.iter().take(depth).map(|x| x[1]).sum::<f64>();
        let imbalance = (bid_pressure - ask_pressure) / (bid_pressure + ask_pressure);
        if imbalance > self.imbalance_threshold && market.price > self.ema_value {
            Signal::BUY
        }
        else if imbalance < -self.imbalance_threshold && market.price < self.ema_value {
            Signal::SELL
        }
        else {
            Signal::HOLD
        }
        
    }
}