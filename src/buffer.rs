/*use std::time::SystemTime;
use crate::data::Order;

pub struct OrderTimeBuffer {
    pub placed_at: SystemTime,
    pub order: Order,
    pub order_id: String,
}

impl OrderTimeBuffer {
    pub fn new(order_: &Order) -> Self {
        Self {
            placed_at: SystemTime::now(),
            order: order_.clone(),
            order_id: order_.order_id.clone(),
        }
    }

    pub fn age(&self) -> Duration {
        SystemTime::now()
        .duration_since(self.placed_at)
        .unwrap_or(Duration::from_secs(0))
    }

    pub fn is_expired(&self, max_age: &Duration) -> bool {
        self.age() >= *max_age
    }
}*/