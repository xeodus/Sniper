use std::sync::mpsc;
use::std::sync::Arc;
use tokio::sync::Mutex;
use std::{collections::BTreeMap, time::Instant};
use std::time::Duration;
use chrono::Utc;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    api_key: String,
    secret_key: String,
    symbols: Vec<String>,
    strategy_params: StrategyParams,
    risk_params: RiskParams,
    paper_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct StrategyParams {
    macd_short: usize,
    macd_long: usize,
    macd_signal: usize,
    rsi_period: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct RiskParams {
    max_drawndown: f64,
    daily_loss_limit: f64,
    max_position_size: f64,
}

#[derive(Debug)]
struct ExchangeConnection {
    websocket_url: String,
    rest_api_url: String,
    client: reqwest::Client,
    rate_limiter: RateLimiter,
}

#[derive(Debug)]
struct RateLimiter {
    requests: u32,
    last_request: Instant,
}

#[derive(Debug)]
struct DataCollect {
    historical_data: Vec<Candle>,
    realtime_data: Option<Candle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Candle {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Debug)]
struct StrategyEngine {
    macd: MACD,
    rsi: RSI,
}

#[derive(Debug)]
struct EMA {
    period: usize,
    alpha: f64,
    current: f64,
    initialized: bool,
}

impl EMA {
    fn new(period: usize) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            alpha,
            current: 0.0,
            initialized: false,
        }
    }
    fn update(&mut self, price: f64) -> f64 {
        if !self.initialized {
            self.current = price;
            self.initialized = true;
        }
        else {
            self.current = self.alpha * price + (1.0 - self.alpha) * self.current;
        }
        self.current
    }
}

#[derive(Debug)]
struct MACD {
    short_ema: EMA,
    long_ema: EMA,
    signal_ema: EMA,
    macd_history: Vec<f64>,
}

#[derive(Debug)]
struct RSI {
    period: usize,
    avg_gain: f64,
    avg_loss: f64,
    last_price: f64,
}

#[derive(Debug)]
struct RiskManager {
    portfolio: Portfolio,
    max_drawdown: f64,
    daily_loss_limit: f64,
    max_position_size: f64,
}

#[derive(Debug)]
struct Portfolio {
    base_currency: f64,
    crypto_position: f64,
    average_entry: f64,
    realized_pnl: f64,
}

#[derive(Debug, Serialize)]
struct TradeLog {
    timestamp: i64,
    side: String,
    quantity: f64,
    price: f64,
    status: String,
}  

fn config() -> Config {
    Config {
        api_key: std::env::var("API_KEY").expect("API key not set!"),
        secret_key: std::env::var("SECRET_KEY").expect("Secret key not set!"),
        symbols: vec!["BTC/USDT".to_string()],
        strategy_params: StrategyParams {
            macd_short: 12,
            macd_long: 26,
            macd_signal: 9,
            rsi_period: 14,
        },
        risk_params: RiskParams {
            max_drawndown: 0.15,
            daily_loss_limit: 500.0,
            max_position_size: 10.0,
        },
        paper_mode: true,
    }
}

impl ExchangeConnection {
    fn connect_exchange() -> Self {
        Self {
            websocket_url: "wss://api.bitget.com/ws".to_string(),
            rest_api_url: "https://api.bitget.com/api/v3".to_string(),
            client: reqwest::Client::new(),
            rate_limiter: RateLimiter {
                requests: 0,
                last_request: Instant::now(),
            }
        }
    }

    async fn check_rate_limiter(&mut self) {
        let elapsed = self.rate_limiter.last_request.elapsed();
        
        while elapsed.as_secs() < 60 {
            if self.rate_limiter.requests >= 100 {
                tokio::time::sleep(Duration::from_secs(60 - elapsed.as_secs())).await;
                self.rate_limiter.requests = 0;
            }
            else {
                self.rate_limiter.requests = 0;
            }
            self.rate_limiter.requests += 1;
            self.rate_limiter.last_request = Instant::now();
        }
    }
}

async fn store_historical_data(exchange: &ExchangeConnection, symbol: &str, timeframe: &str, limit: usize) -> Result<Vec<Candle>, Box<dyn std::error::Error>> {
    let url = format!("{}/kline?symbol={}&timeframe={}&limit={}", exchange.rest_api_url, symbol, timeframe, limit);
    let response = exchange.client.get(&url).send().await.unwrap();
    let status_code = response.status();

    if !status_code.is_success() {
        return Err(format!("API request failed: {:?}", response.text().await?).into());
    }

    let data: Vec<Vec<String>> = response.json().await.unwrap();

    let candles  = data.into_iter().map(|kline| Candle {
        timestamp: kline[0].parse().unwrap_or(0),
        open: kline[1].clone().parse().unwrap(),
        high: kline[2].clone().parse().unwrap(),
        low: kline[3].clone().parse().unwrap(),
        close: kline[4].clone().parse().unwrap(),
        volume: kline[5].clone().parse().unwrap(),
    }).collect();
    Ok(candles)
} 

fn initialized_strategy(config: &Config) -> StrategyEngine {
    StrategyEngine {
        macd: MACD {
            short_ema: EMA::new(config.strategy_params.macd_short),
            long_ema: EMA::new(config.strategy_params.macd_long),
            signal_ema: EMA::new(config.strategy_params.macd_signal),
            macd_history: Vec::new(),
        },
        rsi: RSI {
            period: config.strategy_params.rsi_period,
            avg_gain: 0.0,
            avg_loss: 0.0,
            last_price: 0.0,
        }
    }
}

impl RiskManager {
    fn initialized_risk_manager(config: &Config) -> Self {
        Self {
            portfolio: Portfolio {
                base_currency: 1000.0,
                crypto_position: 0.0,
                average_entry: 0.0,
                realized_pnl: 0.0
            },
            max_drawdown: config.risk_params.max_drawndown,
            daily_loss_limit: config.risk_params.daily_loss_limit,
            max_position_size: config.risk_params.max_position_size,
        }
    }

    fn calculate_position_size(&self, price: f64) -> f64{
        let risk_amount = self.portfolio.total_value() * 0.02;
        (risk_amount / price).min(self.max_position_size)
    }

    fn approve_trade(&self, signal: &str, quantity: f64, price: f64) -> bool {
        match signal {
            "BUY" => self.portfolio.base_currency >= quantity * price,
            "SELL" => self.portfolio.crypto_position >= quantity,
            _ => false
        }
    }

    fn update_portfolio(&mut self, signal: &str, quantity: f64, price: f64) {
        match signal {
            "BUY" => {
                let total_cost = self.portfolio.average_entry * self.portfolio.crypto_position;
                let new_total_cost = total_cost + (quantity * price);
                if self.portfolio.crypto_position > 0.0 {
                    self.portfolio.average_entry = new_total_cost / self.portfolio.crypto_position;
                }
                else {
                    self.portfolio.average_entry = 0.0;
                }
                self.portfolio.crypto_position += quantity;
                self.portfolio.base_currency -= quantity * price;
            },
            "SELL" => {
                self.portfolio.crypto_position -= quantity;
                self.portfolio.base_currency += quantity *price;
                if self.portfolio.crypto_position <= 0.0 {
                    self.portfolio.average_entry = 0.0;
                }
                self.portfolio.realized_pnl += (price - self.portfolio.average_entry) * quantity;
            },
            _ => {}
    }
}
}

impl Portfolio {
    fn total_value(&self) -> f64 {
        self.base_currency + (self.crypto_position * self.average_entry)
    }
}

async fn fetch_realtime_data(exchange: &Arc<Mutex<ExchangeConnection>>, sender: mpsc::Sender<Candle>) -> Result<(), Box<dyn std::error::Error>> {
    let ex = exchange.lock().await;
    let ws_url = &ex.websocket_url;
    let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url).await?;
    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read .next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Ok(candle) = serde_json::from_str::<Candle>(&text) {
                let _ = sender.send(candle);
            }
            else {
                println!("Received non-candle massage: {}", text);
            }
        }
         
    }
    Ok(())
}
async fn execute_trade(config: &Config, exchange: &mut ExchangeConnection, side: &str) -> Result<(), Box<dyn std::error::Error>> {
    if config.paper_mode {
        return log_paper_trade(side).await;
    }
    else {
        return execute_real_trade(config, exchange, side).await;
    }
}
async fn log_paper_trade(side: &str) -> Result<(), Box<dyn std::error::Error>> {
    let log_entry = TradeLog {
        timestamp: Utc::now().timestamp(),
        side: side.to_string(),
        quantity: 0.01,
        price: 89000.0,
        status: "SIMULATED".to_string(),
    };
    println!("Paper trade: {:?}", log_entry);   
    Ok(())
}

async fn execute_real_trade(config: &Config, exchange: &mut ExchangeConnection, side: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut params = BTreeMap::new(); 
    params.insert("timestamp", Utc::now().timestamp_millis().to_string());
    params.insert("side", side.to_uppercase());
    params.insert("type", "MARKET".to_string());
    params.insert("quantity", "0.01".to_string());
    params.insert("symbol", config.symbols[0].to_string());

    let query_string = serde_urlencoded::to_string(&params)?;
    let signature = generate_signature(&config.secret_key, &query_string);
    exchange.check_rate_limiter().await;
    let url = format!("{}/order?{}&signature={}", exchange.rest_api_url, query_string, signature);
    let response = exchange.client.post(&url).header("X-MBX-APIKEY", config.api_key.clone()).send().await?;
    let status_code = response.status();

    if !status_code.is_success() {
        return Err(format!("Invaild order: {:?}", response.text().await?).into());
    }
    println!("Order Successful: {:?}", response.text().await?);
    Ok(())
}

fn generate_signature(secret_key: &str, query: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

async fn main_loop(config: &Config, exchange: &Arc<Mutex<ExchangeConnection>>, strategy: &mut StrategyEngine, risk_manager: &mut RiskManager, candle_receiver: mpsc::Receiver<Candle>, _historical_data: Vec<Candle>) -> Result<(), Box<dyn std::error::Error>> {
    let mut ex = exchange.lock().await;
    let mut data_collector = DataCollect {
        historical_data: store_historical_data(&ex, &config.symbols[0], "1h", 1000).await?,
        realtime_data: None,
    };

    loop {
        let candle = match candle_receiver.recv() {
            Ok(candle) => candle,
            Err(_e) => {
                log::error!("Candle channel closed, exiting main loop.");
                break;
            }
        };

        data_collector.realtime_data = Some(candle.clone());
        data_collector.historical_data.push(candle.clone());

        // Update indicators
        let close_price = candle.close;
        // Update MACD
        let short_ema = strategy.macd.short_ema.update(close_price);
        let long_ema = strategy.macd.long_ema.update(close_price);
        let macd_line = short_ema - long_ema;
        let signal_line = strategy.macd.signal_ema.update(macd_line);
        strategy.macd.macd_history.push(macd_line);

        while close_price != strategy.rsi.last_price {
            // Update RSI
            let price_change = close_price - strategy.rsi.last_price;

            if price_change > 0.0 {
                strategy.rsi.avg_gain = strategy.rsi.avg_gain * (strategy.rsi.period - 1) as f64 + price_change / strategy.rsi.period as f64;
                strategy.rsi.avg_loss = strategy.rsi.avg_loss * (strategy.rsi.period - 1) as f64 / strategy.rsi.period as f64; 
            }
            else {
                strategy.rsi.avg_loss = strategy.rsi.avg_loss * (strategy.rsi.period - 1) as f64 - price_change / strategy.rsi.period as f64;
                strategy.rsi.avg_gain = strategy.rsi.avg_gain * (strategy.rsi.period - 1) as f64 / strategy.rsi.period as f64;
            }

            let rsi = 100.0 - (100.0 / (1.0 + strategy.rsi.avg_gain / strategy.rsi.avg_loss));
            let signal = if macd_line > signal_line && rsi < 30.0 {
                "BUY"
            }
            else if macd_line < signal_line && rsi > 70.0 {
                "SELL"
            }
            else {
                "HOLD"
            };

            let position_size = risk_manager.calculate_position_size(close_price);
            
            if signal != "HOLD" && risk_manager.approve_trade(signal, position_size, close_price) {
                execute_trade(config, &mut ex, signal).await?;
                risk_manager.update_portfolio(signal, position_size, close_price);

                log::info!(
                    "Execute {} order for {} {} @ ${:.2}",
                    signal,
                    close_price,
                    &config.symbols[0],
                    position_size,
                )
            }

        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
    Ok(())
}

#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config();
    let exchange = Arc::new(Mutex::new(ExchangeConnection::connect_exchange()));
    let historical_data = {
        let mut ex = exchange.lock().await;
        store_historical_data(&mut ex, &config.symbols[0], "1h", 1000).await?
    };
    let mut strategy = initialized_strategy(&config);
    let mut risk_manager = RiskManager::initialized_risk_manager(&config);
    let exchange_clone = Arc::clone(&exchange);
    let (candle_sender, candle_receiver) = mpsc::channel::<Candle>();

    tokio::spawn(async move {
        if let Err(e) = fetch_realtime_data(&exchange_clone, candle_sender).await {
            eprint!("Error fetching realtime data: {:?}", e);
        }
    });

    main_loop(&config, &exchange, &mut strategy, &mut risk_manager, candle_receiver, historical_data).await?;

    Ok(())
}
