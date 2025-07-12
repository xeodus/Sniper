use std::{collections::{BTreeMap, HashMap}, env, fs, path::{Path, PathBuf}, time::{Duration, SystemTime, UNIX_EPOCH}};
use csv::WriterBuilder;
use hmac::{Hmac, Mac};
use reqwest::{header::{HeaderMap, HeaderValue, CONTENT_TYPE}, Client};
use sha2::Sha256;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Ok;
use tokio::sync::{broadcast, mpsc};
use crate::ws_stream::{MarketData, WebSocketBuilder};
mod ws_stream;
mod tests;

#[derive(Debug, Clone, Copy)]
pub enum Side {
    BUY,
    SELL
}

#[derive(Debug, Clone, Copy)]
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

pub struct Config {
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub base_url: String,
    pub sandbox: bool
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

#[derive(Debug, Clone)]
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
    symbol: String,
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
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

pub struct DataManager {
    pub base_path: PathBuf
}

impl DataManager {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path)
            .expect("Failed to create data directory..");

        Self { base_path }
    }

    pub fn get_csv_path(&self, symbol: &str, timeframe: i64)
    -> PathBuf
    {
        let symbol_dir = self.base_path.join(symbol.to_uppercase());
        fs::create_dir_all(&symbol_dir).unwrap_or_default();
        symbol_dir.join(format!("{}.csv", timeframe))
    }

    pub fn save_to_csv(&self,
        symbol: &str,
        timeframe: i64,
        candles: &[CandleSticks]
    ) -> Result<PathBuf, anyhow::Error> 
    {
        if candles.is_empty() {
            return Err(anyhow::anyhow!("No data to save.."));
        }

        let file_path = self.get_csv_path(symbol, timeframe);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&file_path)?;

        for data in candles {
            writer.serialize(data)?;
        }

        writer.flush()?;
        println!("Saved {} to {}", candles.len(), file_path.display());
        Ok(file_path)
    }
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
}

trait KucoinFuturesAPI {
    fn new(sandbox: bool) -> Result<Self, anyhow::Error> where Self: Sized;
    async fn signature_generation(&self, timestamp: &str, method: &str, path: &str, body: &str) -> String;
    async fn generate_passphrase(&self) -> String;
    async fn header_assembly(&self, method: &str, path: &str, body: &str) -> HeaderMap;
}

impl KucoinFuturesAPI for Config {
    fn new(sandbox: bool) -> Result<Self, anyhow::Error> {
        let base_url = if sandbox {
            "https://api-sandbox-futures.kucoin.com".into()
        }
        else {
            "https://api-futures.kucoin.com".into()
        };
        Ok(
            Config {
                api_key: env::var("API_KEY").expect("API key not found.."),
                api_secret: env::var("API_SECRET").expect("API secret not found"),
                api_passphrase: env::var("API_PASSPHRASE").expect("API passphrase not found.."),
                base_url,
                sandbox
            }
        )
    }

    async fn signature_generation(&self, timestamp: &str, 
        method: &str, 
        path: &str, 
        body: &str) -> String 
    {
        let query_string = format!("{}{}{}{}", timestamp, method, path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(
            self.api_secret.as_bytes()
        )
        .expect("Hmac can take key of all size..");
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        STANDARD.encode(result.into_bytes())
    }

    async fn generate_passphrase(&self) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(
            self.api_secret.as_bytes()
        )
        .expect("Hmac can take key of all size..");
        mac.update(self.api_passphrase.as_bytes());
        let result = mac.finalize();
        STANDARD.encode(result.into_bytes())
    }

    async fn header_assembly(&self,
        method: &str,
        path: &str, 
        body: &str) -> HeaderMap 
    {
        let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards..")
        .as_secs()
        .to_string();

        let mut headers = HeaderMap::new();
        let signature = self.signature_generation(
            &timestamp,
            method,
            path,
            body)
            .await;
        let passphrase = self.generate_passphrase().await;
        headers.insert("API-KEY", HeaderValue::from_str(&self.api_key).unwrap());
        headers.insert("API-SECRET", HeaderValue::from_str(&signature).unwrap());
        headers.insert("API-PASSPHRASE", HeaderValue::from_str(&passphrase).unwrap());
        headers.insert("API-KEY-TIMESTAMP", HeaderValue::from_str(&timestamp).unwrap());
        headers.insert("API-KEY-VERSION", HeaderValue::from_static("2"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

// STRATEGY

pub struct TechnicalIndicators;

impl TechnicalIndicators {
    pub fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
        let mut sma_values = Vec::new();

        if prices.len() < period {
            return sma_values;
        }
    
        for i in (period - 1).. prices.len() {
            let windows = prices[i - period + 1..=i].iter().sum::<f64>() / period as f64;
            sma_values.push(windows);
        }
        sma_values
    }

    pub fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64> {
        let mut ema_values = Vec::new();
    
        if prices.len() < period {
            return ema_values;
        }
        
        let multiplier = 2.0 / (period + 1) as f64;
        let first_sma = prices[..period].iter().sum::<f64>() / period as f64;
        ema_values.push(first_sma);
    
        for i in period.. prices.len() {
            let prev_ema = ema_values.last().unwrap();
            let ema = (prices[i] - prev_ema) * multiplier + prev_ema;
            ema_values.push(ema);
        }
        ema_values
    }

    pub fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
        let mut rsi_values = Vec::new();
    
        if prices.len() < period + 1 {
            return rsi_values;
        }
    
        let changes: Vec<f64> = prices.windows(2)
            .map(|f| f[1] - f[0]).collect();
    
        let mut gains: Vec<f64> = Vec::new();
        let mut losses: Vec<f64> = Vec::new();
    
        for change in changes {
            gains.push(if change > 0.0 { change } else { 0.0 });
            losses.push(if change < 0.0 { -change } else { 0.0 });
        }
    
        for i in (period - 1).. gains.len() {
            let avg_gain = gains[i - period +1..=i].iter().sum::<f64>() / period as f64;
            let avg_loss = losses[i - period + 1..=i].iter().sum::<f64>() / period as f64;
    
            let rsi = if avg_loss == 0.0 {
                100.0
            }
            else {
                let rs = avg_gain / avg_loss;
                100.0 - (100.0 / (1.0 + rs))
            };
            rsi_values.push(rsi);
        }
        rsi_values
    }

    pub fn calculate_macd(prices: &[f64], 
        fast_period: usize, 
        slow_period: usize, 
        signal_period: usize) -> BTreeMap<String, Vec<f64>> 
    {
        let mut map = BTreeMap::new();
        let fast_ema = Self::calculate_ema(&prices, fast_period);
        let slow_ema = Self::calculate_ema(&prices, slow_period);
    
        if fast_ema.len() < slow_ema.len() {
            map.insert("macd_line".into(), Vec::new());
            map.insert("signal".into(), Vec::new());
            map.insert("histogram".into(), Vec::new());
            return map;
        }
    
        let start_idx = fast_period - slow_period;
    
        let macd_line: Vec<f64> = slow_ema.iter()
            .enumerate()
            .map(|(i, slow)| fast_ema[i + start_idx] - slow)
            .collect();
    
        let signal_line = Self::calculate_ema(&macd_line, signal_period);
    
        let histogram = macd_line.iter()
            .enumerate()
            .skip(macd_line.len() - signal_line.len())
            .zip(signal_line.iter())
            .map(|(macd, signal)| macd.1 - signal)
            .collect();
    
        map.insert("macd_line".into(), macd_line);
        map.insert("signal".into(), signal_line);
        map.insert("histogram".into(), histogram);
        map
    }

    pub fn set_bollinger_bands(prices: &[f64], period: usize, std_multiplier: f64) -> BTreeMap<String, Vec<f64>> {
        let sma = Self::calculate_sma(&prices, period);
        let upper = Vec::new();
        let lower = Vec::new();
        let mut upper_value = Vec::new();
        let mut lower_value = Vec::new();
        let mut bands = BTreeMap::new();
        
        if prices.len() < period {
            bands.insert("middle".to_string(), sma.clone());
            bands.insert("upper".to_string(), upper);
            bands.insert("lower".to_string(), lower);
            return bands;
        }
    
        for i in (period - 1).. prices.len() {
            let windows = &prices[i - period + 1..=i];
            let std_dev: Vec<f64> = windows.iter().map(|x| (x - sma[i - period + 1]).powi(2) as f64).collect();
            let std_dev_sum = std_dev.iter().sum::<f64>();
            upper_value.push(&sma[i - period + 1] + (std_dev_sum * std_multiplier));
            lower_value.push(&sma[i - period + 1] - (std_dev_sum * std_multiplier));
        }
        bands.insert("middle".into(), sma.clone());
        bands.insert("upper".into(), upper_value);
        bands.insert("lower".into(), lower_value);
        bands
    }

}

// Risk Management

pub struct PositionSizer {
    pub account_balance: f64,
    pub risk_per_trade: f64,
    pub max_position_size: f64
}

impl PositionSizer {
    pub fn init(account_balance: f64, risk_per_trade: f64) -> Self {
        Self {
            account_balance,
            risk_per_trade,
            max_position_size: account_balance * 0.1
        }
    }

    pub fn calculate_position_size(&self, entry_price: f64, stop_loss: f64) -> f64 {
        let risk_amount = self.account_balance * self.risk_per_trade;
        let stop_distance = (entry_price - stop_loss).abs();

        if stop_distance == 0.0 {
            return 0.0;
        }

        let position_size = risk_amount / stop_distance;

        return position_size;
    }
}

pub struct PortfolioRiskManager {
    pub max_portfolio_risk: f64,
    pub max_drawdown: f64,
    pub peak_balance: f64
}

impl PortfolioRiskManager {
    pub fn init(max_portfolio_risk: f64, max_drawdown: f64, peak_balance: f64) -> Self {
        Self {
            max_portfolio_risk,
            max_drawdown,
            peak_balance
        }
    }

    pub fn check_portfolio_risk(&self, positions: &[OrderPosition], account_balance: f64) -> bool {
        let margin: Vec<f64> = positions.iter().map(|pos| pos.margin).collect();
        let total_margin = margin.iter().sum::<f64>();
        let portfolio_risk = total_margin / account_balance;
        return portfolio_risk <= self.max_portfolio_risk;
    }

    pub fn check_drawdown(&mut self, current_balance: f64) -> bool {
        self.peak_balance = self.peak_balance.max(current_balance);

        if self.peak_balance == 0.0 {
            return true;
        }

        let drawdown = (self.peak_balance - current_balance) / self.peak_balance;
        return drawdown <= self.max_drawdown;
    }
}

// TRADING STRATEGY

pub trait TradingStrategy {
    fn new(fast_period: usize, slow_period: usize, rsi_period: usize) -> Self;
    fn analyze_market(&mut self, candles: &[CandleSticks]);
    fn should_enter_long(&self) -> bool;
    fn should_enter_short(&self) -> bool;
    fn get_stop_loss(&self, entry_price: f64, side: &Position) -> f64;
    fn get_take_profit(&self, entry_price: f64, side: &Position) -> f64;
    fn should_exit_position(&self, position: &OrderPosition) -> bool;
    fn indicators_ready(&self) -> bool;
}

pub struct MACStrategy {
    pub fast_period: usize,
    pub slow_period: usize,
    pub rsi_period: usize,
    pub rsi_overbought: f64,
    pub rsi_oversold: f64,
    pub indicators: HashMap<String, Vec<f64>>
}

impl TradingStrategy for MACStrategy {

    fn new(fast_period: usize, slow_period: usize, rsi_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            rsi_period,
            rsi_overbought: 70.0,
            rsi_oversold: 30.0,
            indicators: HashMap::new()
        }
    }

    fn analyze_market(&mut self, candles: &[CandleSticks]) {
        if candles.len() < self.slow_period + self.rsi_period { return; }

        let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();
        self.indicators = HashMap::new();
        self.indicators.insert(
            "slow_ma".into(), 
            TechnicalIndicators::calculate_ema(&closes, self.slow_period)
        );
        self.indicators.insert(
            "fast_ma".into(), 
            TechnicalIndicators::calculate_ema(&closes, self.fast_period)
        );
        self.indicators.insert(
            "rsi".into(),
            TechnicalIndicators::calculate_ema(&closes, self.rsi_period)
        );
    }

    fn should_enter_long(&self) -> bool {

        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.indicators.get("slow_ma").unwrap();
        let fast_ma = self.indicators.get("fast_ma").unwrap();
        let rsi = self.indicators.get("rsi").unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 && !rsi.is_empty() {
            let current_golden_cross = fast_ma[fast_ma.len() - 1] > slow_ma[slow_ma.len() - 1];
            let previous_golden_cross = fast_ma[fast_ma.len() - 2] <= slow_ma[slow_ma.len() - 2];
            let rsi_not_overbought = rsi[rsi.len() - 1] < self.rsi_overbought as f64;   
            return current_golden_cross && !previous_golden_cross && rsi_not_overbought;
        }

        false
    }

    fn should_enter_short(&self) -> bool {

        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.indicators.get("slow_ma").unwrap();
        let fast_ma = self.indicators.get("fast_ma").unwrap();
        let rsi = self.indicators.get("rsi").unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 && !rsi.is_empty() {
            let current_golden_cross = fast_ma[fast_ma.len() - 1] < slow_ma[slow_ma.len() - 1];
            let previous_golden_cross = fast_ma[fast_ma.len() - 2] >= slow_ma[slow_ma.len() - 2];
            let rsi_not_overbought = rsi[rsi.len() - 1] > self.rsi_overbought as f64;
            return current_golden_cross && !previous_golden_cross && rsi_not_overbought;
        }

        false
    }

    fn get_stop_loss(&self, entry_price: f64, side: &Position) -> f64 {
        let stop_loss_pct = 0.02;

        match side {
            Position::LONG => {
                return entry_price * (1.0 - stop_loss_pct);
            },
            Position::SHORT => {
                return entry_price * (1.0 + stop_loss_pct);
            }
        }
    }

    fn get_take_profit(&self, entry_price: f64, side: &Position) -> f64 {
        let take_profit_pct = 0.04;

        match side {
            Position::LONG => {
                return entry_price * (1.0 + take_profit_pct);
            },
            Position::SHORT => {
                return entry_price * (1.0 - take_profit_pct);
            }
        }
    }

    fn should_exit_position(&self, positions: &OrderPosition) -> bool {
        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.indicators.get("slow_ma").cloned().unwrap();
        let fast_ma = self.indicators.get("fast_ma").cloned().unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 {

            match positions.position_side {
                Position::LONG => {
                    let current_bearish = fast_ma[fast_ma.len() - 1] < slow_ma[slow_ma.len() - 1];
                    let previous_bullish = fast_ma[fast_ma.len() - 2] >= slow_ma[slow_ma.len() - 2];
                    return current_bearish && previous_bullish;
                },
                Position::SHORT => {
                    let current_bullish = fast_ma[fast_ma.len() - 1] > slow_ma[slow_ma.len() - 1];
                    let previous_bearish = fast_ma[fast_ma.len() - 2] <= slow_ma[slow_ma.len() - 2];
                    return current_bullish && previous_bearish;
                }
            }
        }

        false
    }

    fn indicators_ready(&self) -> bool {
        let required_indicators = ["fast_ma", "slow_ma", "rsi"];
        
        for indicator in required_indicators {
            if !self.indicators[indicator].is_empty() {
                return false;
            }
        }
        true
    }
    
}

// TRADE BOT ENGINE

pub struct TradingEngine {
    pub config: Config,
    pub strategy: MACStrategy,
    pub position_size: PositionSizer,
    pub data_manager: DataManager,
    pub client: Client,
    pub active_position: HashMap<String, OrderPosition>,
    pub market_data_rx: mpsc::Receiver<MarketData>
}

impl TradingEngine {
    pub fn new(config: Config,
        strategy: MACStrategy,
        account_balance: f64,
        risk_per_trade: f64,
        data_path: &str,
        market_data_rx: mpsc::Receiver<MarketData>
    ) -> Self
    {
        Self {
            config,
            strategy,
            position_size: PositionSizer::init(account_balance, risk_per_trade),
            data_manager: DataManager::new(data_path),
            client: Client::new(),
            active_position: HashMap::new(),
            market_data_rx 
        }
    }

    pub fn get_timestamp() -> String {
        SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string()
    }

    pub async fn get_klines(&self, query: &KlineQuery) -> Result<Vec<CandleSticks>, anyhow::Error> {
        let timestamp = Self::get_timestamp();
        let path = format!(
            "/api/v3/kline/query?symbol={}&granularity={}&from={}&to={}",
            query.symbol, query.interval, query.from_time, query.to_time
        );
        let headers = self.config.header_assembly("GET", &path, &timestamp).await;
        let url = format!("{}/{}", self.config.base_url, path);
        let response = self.client.get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(format!("Invalid response received: {}", response.text().await?)).into());
        }

        let json: Value = response.json().await?;
        let mut candles = Vec::new();

        if let Some(data) = json["data"].as_array() {
            for item in data {
                if let Some(arr) = item.as_array() {
                    if arr.len() >= 6 {
                        let candle = CandleSticks {
                            symbol: query.symbol.clone(),
                            timestamp: arr[0].as_str().unwrap_or("0").parse()?,
                            open: arr[1].as_str().unwrap_or("0").parse()?,
                            high: arr[2].as_str().unwrap_or("0").parse()?,
                            low: arr[3].as_str().unwrap_or("0").parse()?,
                            close: arr[4].as_str().unwrap_or("0").parse()?,
                            volume: arr[5].as_str().unwrap_or("0").parse()?
                        };
                        candles.push(candle);
                    }
                }
            }
        }

        Ok(candles)
    }

    pub async fn run_strategy(&mut self, symbol: &str, interval: &str) -> Result<(), anyhow::Error> {
        println!("Starting the bot for symbol {} with interval {}", symbol, interval);

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let one_hour_ago = timestamp - 3600;

        let query = KlineQuery {
            symbol: symbol.to_string(),
            from_time: one_hour_ago,
            to_time: timestamp,
            limit: Some(100),
            interval: interval.to_string()
        };

        let candles = self.get_klines(&query).await
            .map_err(|e| anyhow::anyhow!(format!("Unable to fetch candle sticks for historical data: {}", e)))?;

        self.strategy.analyze_market(&candles);

        if self.strategy.should_enter_long() {
            println!("Long signal received for symbol {}", symbol);
        }

        if self.strategy.should_enter_short() {
            println!("Short signal received for symbol {}", symbol);
        }

        if let Some(position) = self.active_position.get(symbol) {
            if self.strategy.should_exit_position(position) {
                println!("Exit signal detected for symbol {}", symbol);
            }
        }

        if let Err(e) = self.data_manager.save_to_csv(symbol, timestamp, &candles) {
            eprintln!("Failed to save data to csv: {}", e);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    let (bcast_tx, _) = broadcast::channel::<MarketData>(100);
    let (mpsc_tx, mpsc_rx) = mpsc::channel::<MarketData>(100);
    let mut bcast_rx = bcast_tx.subscribe();

    tokio::spawn(async move {
        let data = bcast_rx.recv().await
            .map_err(|e| format!("Failed to fetch market data: {}", e));
        if let Err(err) = mpsc_tx.send(data.unwrap()).await {
            eprintln!("Failed to forward market data: {}", err);
        }
    });

    let ws = WebSocketBuilder::new("wss://your.websocket.url".into(), bcast_tx);
    let symbols = vec!["ETHUSDT".into()];

    tokio::spawn(async move{
        if let Err(e) = ws.ws_connect(&symbols).await {
            eprintln!("WebSocket connection failed: {}", e);
        }
    });

    let config = Config::new(true)?;
    let strategy = Box::new(TradingStrategy::new(12, 26, 14));
    let mut bot = TradingEngine::new(config, *strategy, 10000.0, 0.02, "./trading_data", mpsc_rx);
    bot.run_strategy("ETHUSDT", "1min").await?;
    Ok(())
}