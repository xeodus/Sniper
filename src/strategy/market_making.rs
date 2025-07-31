use uuid::Uuid;
use crate::data::*;

pub struct MM;

impl MM {
    pub fn new() -> Self { MM }

    pub fn decide(&mut self, tob: &TopOfBook) -> Option<OrderReq> {
        let spread = (tob.ask - tob.bid) / tob.bid;
        if spread < 0.001 { return None; }
        let mid = (tob.bid + tob.ask) / 2.0;
        let target = 0.01;

        return Some(OrderReq {
            id: Uuid::new_v4().to_string(),
            symbol: tob.symbol.clone(),
            price: mid * (1.0 + target / 2.0),
            quantity: 0.001,
            side : Side::Buy
        }).or(Some(OrderReq {
            id: Uuid::new_v4().to_string(),
            symbol: tob.symbol.clone(),
            price: mid * (1.0 - target / 2.0),
            quantity: 0.001,
            side: Side::Sell
        }));
    }
}
