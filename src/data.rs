#[derive(Debug)]
pub enum Side {
    Buy,
    Sell
}

pub enum Exchange {
    KuCoin,
    Binance
}

/*pub enum OrderStatus {
    New,
    Filled,
    Canceled,
    Rejected
}*/

#[derive(Debug)]
pub struct OrderReq {
    pub id: String,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub timestamp: i64
}

pub struct TopOfBook {
    pub exchange: Exchange,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64
}

/*pub struct OrderUpdate {
    pub id: String,
    pub status: OrderStatus
}*/

pub struct TechnicalIndicators;
