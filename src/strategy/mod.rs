use std::collections::VecDeque;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MarketData {
    price: f64,
    quantity: f64,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>
}

#[derive(PartialEq)]
pub enum Signal {
    BUY,
    SELL,
    HOLD
}

#[derive(Debug, Deserialize)]
pub struct TradeState {
    order_book_depth: usize,
    imbalance_threshold: f64,
    entry_price: f64,
    ema_period: usize,
    ema_value: f64,
    ema_count: usize,
    price_buffer: VecDeque<f64>, // Initializes the sma by storing market prices
    max_position: f64,
    stop_loss: f64,
}

impl TradeState {
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
        let alpha = 2.0 / self.ema_period as f64 + 1.0;

        if self.price_buffer.len() < self.ema_period {
            self.price_buffer.push_back(market.price);
            self.ema_count += 1;
        }
        else if self.price_buffer.len() == self.ema_period {
            let sma = self.price_buffer.iter().sum::<f64>() / self.ema_period as f64 + 1.0;
            self.ema_value = sma;
        }
        else {
            self.ema_value = (market.price * alpha) + (1.0 - alpha) * self.ema_value;
        }

        return self.ema_value;
    }

    fn generate_signal(&mut self, market: &MarketData, depth: usize) -> Signal {
        let bid_pressure = market.bids.iter().take(depth).map(|(p,q)| p * q).sum::<f64>();
        let ask_pressure = market.asks.iter().take(depth).map(|(p, q)| p * q).sum::<f64>();
        let imbalance = (bid_pressure - ask_pressure) / (bid_pressure + ask_pressure);

        if imbalance < self.imbalance_threshold && market.price < self.ema_value {
            return Signal::BUY;
        }
        else if imbalance > -self.imbalance_threshold && market.price > self.ema_value {
            return Signal::SELL;
        }
        else {
            return Signal::HOLD;
        }
    }
}