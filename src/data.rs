use serde::Deserialize;
pub use crate::indicators::TechnicalIndicators;

#[derive(Debug, Deserialize, Clone)]
pub enum Side {
    Buy,
    Sell
}

#[derive(Debug, Deserialize, Clone)]
pub enum Exchange {
    KuCoin,
    Binance
}

#[derive(Debug, Deserialize, Clone)]
pub enum OrderStatus {
    New,
    Filled,
    Rejected
}
#[derive(Debug, Deserialize, Clone)]
pub struct OrderReq {
    pub id: String,
    pub exchange: Exchange,
    pub symbol: String,
    pub type_: String,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct Candles {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64
}

pub struct TrendDetector {
    pub alpha_slow: f64,
    pub alpha_fast: f64,
    pub alpha_atr: f64,
    pub ema_slow: f64,
    pub ema_fast: f64,
    pub atr: f64,
    pub prev_closed: f64,
    pub initialized: bool,
    pub k_atr: f64
}

#[derive(Debug, Clone)]
pub struct GridOrder {
    pub client_oid: String,
    pub symbol: String,
    pub level: f64,
    pub side: Side,
    pub active: bool,
    pub quantity: f64,
    pub status: OrderStatus
}

#[derive(Debug, Clone)]
pub struct BotState {
    pub trend: Trend,
    pub open_orders: Vec<GridOrder>,
    pub pnl: f64
}

/*pub struct OrderUpdate {
    pub id: String,
    pub status: OrderStatus
}*/

#[derive(Debug, Clone)]
pub enum Trend {
    UpTrend,
    DownTrend,
    SideChop
}
