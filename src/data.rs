use std::{collections::HashMap, path::PathBuf, time::SystemTime};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::ws_stream::MarketData;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Side {
    BUY,
    SELL
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum OrderType {
    MARKET,
    LIMIT,
    STOP
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Position {
    LONG,
    SHORT
}

pub struct KuCoinGateway {
    pub cfg: Config,
    pub client: Client
}

pub struct DataManager {
    pub base_path: PathBuf
}

pub struct PositionSizer {
    pub account_balance: f64,
    pub risk_per_trade: f64,
    pub max_position_size: f64
}

pub struct PortfolioRiskManager {
    pub max_portfolio_risk: f64,
    pub max_drawdown: f64,
    pub peak_balance: f64
}

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub base_url: String,
    pub sandbox: bool,
    pub risk_per_trade: f64,
    pub account_balance: f64,
    pub max_portfolio_risk: f64,
    pub max_drawdown: f64
}

pub struct MACStrategy {
    pub fast_period: usize,
    pub slow_period: usize,
    pub rsi_period: usize,
    pub rsi_overbought: f64,
    pub rsi_oversold: f64,
    pub indicators: HashMap<String, Vec<f64>>
}

pub struct TradingEngine {
    pub config: Config,
    pub strategy: MACStrategy,
    pub position_size: PositionSizer,
    pub data_manager: DataManager,
    pub client: Client,
    pub gateway: KuCoinGateway,
    pub active_position: HashMap<String, OrderPosition>,
    pub market_data_rx: broadcast::Receiver<MarketData>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderPosition {
    pub symbol: String,
    pub position_side: Position,
    pub size: f64,
    pub entry_price: f64,
    pub available_balance: f64,
    pub market_price: f64,
    pub stop_loss: f64,
    pub stop_price: f64,
    pub unrealised_pnl: f64,
    pub margin: f64
}

#[derive(Debug, Deserialize, Clone)]
pub struct Order {
    pub order_id: String,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub size: f64,
    pub price: f64,
    pub status: String,
    pub filled_size: f64,
    pub timestamp: i64
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandleSticks {
    pub symbol: String,
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

pub struct KlineQuery {
    pub symbol: String,
    pub from_time: i64,
    pub to_time: i64,
    pub limit: Option<i32>,
    pub interval: String
}

pub struct OrderTimeBuffer {
    pub placed_at: SystemTime,
    pub order: Order,
    pub order_id: String,
}