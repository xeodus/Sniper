use std::sync::Arc;
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::{mpsc, RwLock};
use crate::{db::Database, position_manager::PositionManager, 
    rest_client::BinanceClient, signal::MarketSignal};

#[derive(Debug, Clone)]
pub enum PositionSide {
    Long,
    Short
}

#[derive(Debug, Clone, PartialEq)]
pub enum Side {
    Buy,
    Sell,
    Hold
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit
}

#[derive(Debug, Clone, PartialEq)] 
pub enum Trend {
    UpTrend,
    DownTrend,
    Sideways
}

#[derive(Debug, Clone)]
pub struct Position {
    pub id: String,
    pub symbol: String,
    pub position_side: PositionSide,
    pub entry_price: Decimal,
    pub size: Decimal,
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
    pub opened_at: i64
}

#[derive(Debug)]
pub struct Candles {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub timestamp: i64
}

#[derive(Debug, Clone)]
pub struct OrderReq {
    pub id: String,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Decimal,
    pub size: Decimal,
    pub sl: Option<Decimal>,
    pub tp: Option<Decimal>,
    pub manual: bool
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub timestamp: i64,
    pub symbol: String,
    pub action: Side,
    pub trend: Trend,
    pub price: Decimal,
    pub confidence: f64
}

pub struct TradingBot {
    pub analyzer: Arc<RwLock<MarketSignal>>,
    pub position_manager: Arc<PositionManager>,
    pub binance_client: Arc<BinanceClient>,
    pub signal_tx: mpsc::Sender<Signal>,
    pub order_tx: mpsc::Sender<OrderReq>,
    pub account_balace: Arc<RwLock<Decimal>>,
    pub db: Arc<Database>
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceKline {
    #[serde(rename="t")]
    pub open_time: i64,
    #[serde(rename="o")]
    pub open: String,
    #[serde(rename="h")]
    pub high: String,
    #[serde(rename="l")]
    pub low: String,
    #[serde(rename="c")]
    pub close: String,
    #[serde(rename="v")]
    pub volume: String
}

/*#[derive(Debug, Clone, Deserialize)]
pub struct BinanceKlineEvent {
    #[serde(rename="e")]
    pub event_type: String,
    #[serde(rename="k")]
    pub kline: BinanceKline
}*/
