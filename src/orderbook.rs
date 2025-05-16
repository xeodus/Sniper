use crate::market_stream::{DepthSnapshot, DepthUpdate, OrderBookLevel};

pub struct OrderBook {
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
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
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        // Set asks in ascending order
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    }

    fn apply_updates(&mut self, updates: &DepthUpdate) -> bool {
        if updates.final_update_id <= self.last_update_id {
            return false;
        }

        for bid in updates.bids.iter() {
            if bid.quantity == 0.0 {
                self.bids.retain(|x| x.price != bid.price);
            }
            else {
                if let Some(existing) = self.bids.iter_mut()
                .find(|x| x.price == bid.price) {
                    existing.quantity = bid.quantity;
                }
                else {
                    self.bids.push(OrderBookLevel { price: bid.price, quantity: bid.quantity });
                }
                self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
            }
        }

        for ask in updates.asks.iter() {
            if ask.quantity == 0.0 {
                self.asks.retain(|x| x.price != ask.price);
            }
            else {
                if let Some(existing) = self.asks.iter_mut()
                .find(|x| x.price == ask.price) {
                    existing.price = ask.price;
                }
                else {
                    self.asks.push(OrderBookLevel { price: ask.price, quantity: ask.quantity });
                }
                self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            }
        }

        self.last_update_id = updates.final_update_id;
        true
    }

    fn best_ask(&self) -> f64 {
        self.asks.first().map_or(0.0, |ask| ask.price)
    }

    fn best_bid(&self) -> f64 {
        self.bids.first().map_or(0.0, |bid| bid.price)
    }                      
    
    fn mid_price(&self) -> f64 {
        (self.best_bid() + self.best_ask()) / 2.0
    }
}