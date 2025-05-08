use crate::market_stream::{DepthSnapshot, DepthUpdate, OrderBookLevel};

pub struct OrderBook {
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub last_update_id: u64
}

pub trait OrderBookManager {
    fn initialize(&self) -> Self;
    fn apply_snapshots(&mut self, snapshot: &DepthSnapshot);
    fn apply_updates(&mut self, updates: &DepthUpdate);
    fn best_bid(&self) -> f64;
    fn best_ask(&self) -> f64;
}

impl OrderBookManager for OrderBook {
    fn initialize(&self) -> Self {
        Self {
            bids: self.bids.clone(),
            asks: self.asks.clone(),
            last_update_id: self.last_update_id
        }
    }

    fn apply_snapshots(&mut self, snapshot: &DepthSnapshot) {
        self.bids = snapshot.bids.clone();
        self.asks = snapshot.asks.clone();
        self.last_update_id = snapshot.last_updated_id;
    }

    fn apply_updates(&mut self, updates: &DepthUpdate) {
        self.bids = updates.bids.clone();
        self.asks = updates.asks.clone();
        self.last_update_id = updates.final_update_id;
    }

    fn best_bid(&self) -> f64 {
        self.bids[0].price
    }

    fn best_ask(&self) -> f64 {
        self.asks[0].price
    }
}