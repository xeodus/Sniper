use crate::market_stream::{DepthSnapshot, DepthUpdate};

pub struct OrderBook {
    pub bids: Vec<[f64; 2]>,
    pub asks: Vec<[f64; 2]>,
    pub last_update_id: u64
}

pub trait OrderBookManager {
    fn initialize() -> Self;
    fn apply_snapshots(&mut self, snapshot: &DepthSnapshot);
    fn apply_updates(&mut self, updates: &DepthUpdate) -> bool;
    fn best_bid(&self) -> f64;
    fn best_ask(&self) -> f64;
    fn mid_price(&self) -> f64;
}

impl OrderBookManager for OrderBook {
    fn initialize() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
            last_update_id: 0
        }
    }

    fn apply_snapshots(&mut self, snapshot: &DepthSnapshot) {
        self.bids = snapshot.bids.clone();
        self.asks = snapshot.asks.clone();
        self.last_update_id = snapshot.last_updated_id;
        // Set bids in descending order
        self.bids.sort_by(|a, b| b[0].partial_cmp(&a[0]).unwrap());
        // Set asks in ascending order
        self.asks.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
    }

    fn apply_updates(&mut self, updates: &DepthUpdate) -> bool {

        if updates.final_update_id <= self.last_update_id {
            return false;
        }

        for &[price, quantity] in updates.bids.iter() {
            if price == 0.0 {
                self.bids.retain(|x| x[0] != price);
            }
            else {
                if let Some(existing) = self.bids.iter_mut()
                .find(|x| x[0] == price) {
                    existing[1] = quantity;
                }
                else {
                    self.bids.push([price, quantity]);
                }
                self.bids.sort_by(|a, b| b[0].partial_cmp(&a[0]).unwrap());
            }
        }

        for &[price, quantity] in updates.asks.iter() {
            if quantity == 0.0 {
                self.asks.retain(|x| x[0] != price);
            }
            else {
                if let Some(existing) = self.asks.iter_mut()
                .find(|x| x[0] == price) {
                    existing[0] = price;
                }
                else {
                    self.asks.push([price, quantity]);
                }
                self.asks.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
            }
        }

        self.last_update_id = updates.final_update_id;
        true
    }

    fn best_ask(&self) -> f64 {
        self.asks.first().map_or(0.0, |ask| ask[0])
    }

    fn best_bid(&self) -> f64 {
        self.bids.first().map_or(0.0, |bid| bid[0])
    }                      
    
    fn mid_price(&self) -> f64 {
        (self.best_bid() + self.best_ask()) / 2.0
    }
}