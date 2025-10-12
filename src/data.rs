#[derive(Debug, Clone)]
pub enum Side {
    Buy,
    Sell
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    New,
    Filled,
    Rejected
}

#[derive(Debug, Clone)]
pub enum Exchange {
    Binance,
    KuCoin
}

#[derive(Debug, Clone, PartialEq)]
pub enum Trend {
    UpTrend,
    DownTrend,
    SideChop
}

#[derive(Debug, Clone)]
pub struct OrderReq {
    pub id: String,
    pub symbol: String,
    pub exchange: Exchange,
    pub price: f64,
    pub size: f64,
    pub type_: String,
    pub side: Side,
    pub timestamp: i64
}

#[derive(Debug, Clone)]
pub struct Candles {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: i64
}

#[derive(Debug, Clone)]
pub struct GridOrder {
    pub client_oid: String,
    pub symbol: String,
    pub level: f64,
    pub size: f64,
    pub active: bool,
    pub side: Side,
    pub status: OrderStatus
}

#[derive(Debug)]
pub struct GridStrategy {
    pub grid_levels: Vec<f64>,
    pub active_orders: Vec<GridOrder>,
    pub center_price: f64,
    pub grid_spacing: f64,
    pub max_levels: usize
}

pub struct TechnicalIndicators;

#[derive(Debug)]
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
pub struct BotState {
    pub trend: Trend,
    pub open_orders: Vec<GridOrder>,
    pub pnl: f64
}

