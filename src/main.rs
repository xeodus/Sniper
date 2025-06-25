use std::{collections::{hash_set::SymmetricDifference, BTreeMap, HashMap}, env, fs, path::{Path, PathBuf}, time::{self, Duration, SystemTime, UNIX_EPOCH}};
use hmac::{Hmac, Mac};
use reqwest::{header::{HeaderMap, HeaderValue, CONTENT_TYPE}, Client};
use sha2::Sha256;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;
use anyhow::anyhow;

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

struct Config {
    api_key: String,
    api_secret: String,
    api_passphrase: String,
    base_url: String,
    session: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct OrderPosition {
    symbol: String,
    position_side: Position,
    size: f64,
    entry_price: f64,
    available_balance: f64,
    market_price: f64,
    stop_loss: f64,
    stop_price: f64,
    unrealised_pnl: f64,
    margin: f64
}

#[derive(Debug, Clone)]
struct Order {
    order_id: String,
    symbol: String,
    side: Side,
    order_type: OrderType,
    size: f64,
    price: f64,
    status: String,
    filled_size: f64,
    timestamp: i64
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CandleSticks {
    symbol: String,
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    quote_volume: f64,
    trades_count: Option<i32>,
    taker_buy_volume: Option<f64>,
    taker_buy_quote_volume: Option<f64>
}

struct KlineQuery {
    symbol: String,
    from_time: i64,
    to_time: i64,
    limit: Option<i32>
}

struct OrderTimeBuffer {
    placed_at: SystemTime,
    order: Order,
    order_id: String,
}

struct DataManager {
    base_path: PathBuf
}

impl DataManager {
    fn new<p: AsRef<Path>>(base_path: p) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path)
            .expect("Failed to create data directory..");

        Ok(Self { base_path })
            
    }

    fn get_csv_path(&self, symbol: &str, timeframe: &str)
    -> PathBuf
    {
        self.base_path.join(symbol.to_uppercase())
        .join(foramt!("{}.csv", timeframe))
    }

    fn save_to_csv(&self,
        symbol: &str,
        timeframe: &str,
        candles: &[CandleSticks]
    ) -> Result<PathBuf> 
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
    fn new(order_: &Order) -> Self {
        Self {
            placed_at: SystemTime::now(),
            order: order_.clone(),
            order_id: order_.order_id.clone(),
        }
    }

    fn age(&self) -> Duration {
        SystemTime::now()
        .duration_since(self.placed_at)
        .unwrap_or(Duration::from_secs(0))
    }

    fn is_expired(&self, max_age: &Duration) -> bool {
        self.age() >= *max_age
    }
}

trait KucoinFuturesAPI {
    fn new() -> Self;
    async fn signature_generation(&self, query_string: &str) -> String;
    async fn generate_passphrase(&self) -> String;
    async fn header_assembly(&self, endpoint: &str) -> HeaderMap;
    async fn authenticate_request(
        &mut self,
        pos_: &OrderPosition,
        type_: &OrderType
    ) -> Result<String, Box<dyn std::error::Error>>;
}

impl KucoinFuturesAPI for Config {
    fn new() -> Self {
        Config {
            api_key: env::var("API_KEY").expect("API key not found.."),
            api_secret: env::var("API_SECRET").expect("API secret not found"),
            api_passphrase: env::var("API_PASSPHRASE").expect("API passphrase not found.."),
            base_url: "https://api-sandbox-futures.kucoin.com".to_string(),
            session: None
        }
    }

    async fn signature_generation(&self, query_string: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(
            self.api_secret.as_bytes()
        )
        .expect("Hmac can take key of all size..");
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        STANDARD.encode(result.into_bytes())
    }

    async fn generate_passphrase(&self) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(
            self.api_secret.as_bytes()
        )
        .expect("Hmac can take key of all size..");
        mac.update(self.api_passphrase.as_bytes());
        let result = mac.finalize();
        STANDARD.encode(result.into_bytes())
    }

    async fn header_assembly(&self, endpoint: &str) -> HeaderMap {
        let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards..")
        .as_secs()
        .to_string();

        let mut headers = HeaderMap::new();
        let signature = self.signature_generation(endpoint).await;
        let passphrase = self.generate_passphrase().await;
        headers.insert("KC-API-KEY", HeaderValue::from_str(&self.api_key).unwrap());
        headers.insert("KC-API-SECRET", HeaderValue::from_str(&signature).unwrap());
        headers.insert("KC-API-PASSPHRASE", HeaderValue::from_str(&passphrase).unwrap());
        headers.insert("KC-API-KEY-TIMESTAMP", HeaderValue::from_str(&timestamp).unwrap());
        headers.insert("KC-API-KEY-VERSION", HeaderValue::from_static("2"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    async  fn authenticate_request(
        &mut self, 
        pos_: &OrderPosition, 
        type_: &OrderType
    ) -> Result<String, Box<dyn std::error::Error>> {

        let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards..")
        .as_secs()
        .to_string();

        let mut params = HashMap::new();
        params.insert("symbol", pos_.symbol.clone());

        let type_str = match type_ {
            OrderType::MARKET => "MARKET".to_string(),
            OrderType::LIMIT => "LIMIT".to_string(),
            OrderType::STOP => "STOP".to_string()
        };

        let order_type = match type_str.as_str() {
            "MARKET" => {
                println!("Placed market order for buying!");
                params.insert("order_type", pos_.market_price.to_string())
            },
            "LIMIT" => {
                println!("Placed limit order for buying!");
                params.insert("order_type", pos_.entry_price.to_string())
            },
            "STOP" => {
                println!("Placed stop order for buying!");
                params.insert("order_type", pos_.stop_price.to_string())
            }
            _ => None
        };

        params.insert("order_type", format!("{:?}", order_type));

        let query_string = serde_urlencoded::to_string(params).unwrap();
        let client = Client::new();
        let url = format!("{}/api/v1/?query_string{}&timestamp={}",
            self.base_url,
            query_string,
            timestamp
        );
        let header = self.header_assembly(&url).await;

        let response = client.get(&url)
        .headers(header)
        .send()
        .await?;

        let res_status = response.status();

        if !res_status.is_success() {
            return Err(format!(
                "Invalid response received: {}",
                response.text().await?)
                .into()
            );
        }

        let body = response.text().await?;

        Ok(body)
    }
}

fn get_timestamp() -> String {
    let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("Time ran backwards..")
    .as_secs()
    .to_string();
    
    return timestamp;
}

fn get_order_id(response_json: &Value, order_: &Order) -> String {
    let order_id = match &response_json["order-id"] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return format!("Cannot fetch the order ID for: {}", order_.order_id).into()
    };

    return order_id;
}

async fn get_account_info(
    pos_: &OrderPosition,
    cfg: &Config
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>
{
    let timestamp = get_timestamp();

    let client = Client::new();
    let url = format!(
        "{}/api/v1/?symbol{}&balance={}&position={}&timestamp={}",
        cfg.base_url,
        pos_.symbol,
        pos_.available_balance,
        pos_.size,
        timestamp
    );
    let response = client.get(&url)
        .send()
        .await?
        .json::<OrderPosition>()
        .await?;

    let body = serde_json::to_value(response)?;
    let value = vec![body];
    Ok(value)
}

async fn place_buy_order(
    type_: &OrderType,
    pos_: &OrderPosition,
    order_: &Order,
    cfg: &Config
) -> Result<String, Box<dyn std::error::Error>> 
{
    let timestamp = get_timestamp();

    let mut params = HashMap::new();
    params.insert("symbol", order_.symbol.clone());
    params.insert("quantity", order_.size.to_string());
    params.insert("side", "BUY".to_string());

    params.insert("stop_loss", pos_.stop_loss.to_string());

    let type_str = match type_ {
        OrderType::MARKET => "MARKET".to_string(),
        OrderType::LIMIT => "LIMIT".to_string(),
        OrderType::STOP => "STOP".to_string()
    };

    let order_type = match type_str.as_str() {
        "MARKET" => {
            println!("Placed market order for buying!");
            params.insert("order_type", pos_.market_price.to_string())
        },
        "LIMIT" => {
            println!("Placed limit order for buying!");
            params.insert("order_type", pos_.entry_price.to_string())
        },
        "STOP" => {
            println!("Placed stop order for buying!");
            params.insert("order_type", pos_.stop_price.to_string())
        },
        _ => {
            None
        }
    };

    params.insert("order_type", format!("{:?}", order_type));

    let client = Client::new();
    let query_string = serde_urlencoded::to_string(params).unwrap();
    let url = format!("{}/api/v1/?query_string{}&timestamp={}", cfg.base_url, query_string, timestamp);
    let response = client.post(&url)
        .send()
        .await?;

    let res_status = response.status();

    if !res_status.is_success() {
        return Err(format!("Invalid response received: {}", response.text().await?).into());
    }

    let response_json = response.json::<Value>().await?;

   let order_id = get_order_id(&response_json, &order_);

    Ok(order_id)
}

async fn place_sell_order(
    type_: &OrderType,
    pos_: &OrderPosition,
    order_: &Order,
    cfg: &Config
) -> Result<String, Box<dyn std::error::Error>>
{
    let timestamp = get_timestamp();

    let mut params = HashMap::new();
    params.insert("symbol", order_.symbol.clone());
    params.insert("quantity", order_.size.to_string());
    params.insert("side", "SELL".to_string());

    let type_str = match type_ {
        OrderType::MARKET => "MARKET".to_string(),
        OrderType::LIMIT => "LIMIT".to_string(),
        OrderType::STOP => "STOP".to_string()
    };
    
    let order_type = match type_str.as_str() {
        "MARKET" => {
            println!("Placed market order for buying!");
            params.insert("order_type", pos_.market_price.to_string())
        },
        "LIMIT" => {
            println!("Placed limit order for buying!");
            params.insert("order_type", pos_.entry_price.to_string())
        },
        "STOP" => {
            println!("Placed stop order for buying!");
            params.insert("order_type", pos_.stop_price.to_string())
        },
        _ => {
            None
        }
    };

    params.insert("order_type", format!("{:?}", order_type));

    let client = Client::new();
    let query_string = serde_urlencoded::to_string(params).unwrap();
    let url = format!("{}/api/v1/?query_string{}&timestamp={}",
        cfg.base_url,
        query_string,
        timestamp
    );

    let response = client.post(&url)
        .send()
        .await?;

    let res_status = response.status();

    if !res_status.is_success() {
        return Err(format!("Invalid response received: {}", response.text().await?).into());
    }

    let response_json = response.json::<Value>().await?;

    if pos_.market_price < pos_.entry_price {
        let loss_amount = pos_.entry_price - pos_.market_price;
        let loss_percentage = (loss_amount / pos_.entry_price) * 100.0;
        eprintln!("Execute the sell order at {:.1}% loss for: {}", loss_percentage, order_.symbol);
    }

    if order_.size == 0.0 {
        return Err(format!("Invalid order size for: {}", order_.symbol).into());
    }

    let order_id = get_order_id(&response_json, &order_);

    Ok(order_id)
}

async fn cancel_order(order_: &Order, cfg: &Config)
-> Result<bool, Box<dyn std::error::Error>>
{
    let timestamp = get_timestamp();

    if order_.filled_size <= 0.0 || order_.size <= 0.0 {
        eprintln!("Invaild amount cannot place order for: {}", order_.symbol);
        return Ok::<bool, Box<dyn std::error::Error>>(true);
    }

    let client = Client::new();
    let endpoint = format!("{}/api/v1/?symbol{}&order_id={}&timestamp={}",
        cfg.base_url,
        order_.symbol,
        order_.order_id,
        timestamp
    );

    let response = client.post(&endpoint)
        .send()
        .await?;

    let res_status = response.status();

    if !res_status.is_success() {
        println!("Cannot place the order invalid response received..");
        Ok(true)
    }
    else {
        println!("Cannot cancel the order..");
        Ok(false)
    }
}

async fn get_status(order_: &Order, cfg: &Config)
-> Result<String, Box<dyn std::error::Error>>
{
    let timestamp = get_timestamp();

    let client = Client::new();
    let url = format!("{}/api/v1/status?symbol{}&order_id={}&timestamp={}",
        cfg.base_url, 
        order_.symbol, 
        order_.order_id, 
        timestamp
    );

    let response = client.get(&url).send().await?;
    let res_status = response.status();

    if !res_status.is_success() {
        return Err(format!("Invalid response received for: {}", order_.symbol).into());
    }

    let response_json = response.json::<Value>().await?;

    let order_status = &response_json["status"]
    .as_str()
    .unwrap_or("UNKNOWN")
    .to_string();

    Ok(order_status.to_string())
}

async fn check_and_cancel(
    order_: &Order,
    cfg: &Config,
    max_buffer: &OrderTimeBuffer
)
-> Result<bool, Box<dyn std::error::Error>>
{
    let order_status = get_status(&order_, &cfg).await?;

    if max_buffer.is_expired(&Duration::from_secs(86400)) {
        return Ok(false);
    }

    match order_status.as_str() {
        "FILLED" | "EXPIRED" | "UNFILLED" | "CANCELED" => {
            println!(
                "Order {} with order status {} is already canceled, no need to cancel",
                order_.order_id,
                order_status
            );
            return Ok(false);
        },
        "PARTIALLY FILLED" | "NEW" => {
            println!(
                "Order {} with order status {} with age {:?} found, initiating cancellation..",
                order_.order_id,
                order_status,
                max_buffer.age()
            );
        }
        _ => {
            println!("Unkown order status found for order {}, cancelling anyway..", order_.order_id);
        }
    }

    cancel_order(&order_, &cfg).await
}

async fn auto_cancel_orders(
    order_: &Order,
    cfg: &Config,
    max_buffer: &OrderTimeBuffer
) -> Result<bool, Box<dyn std::error::Error>>
{
    sleep(Duration::from_secs(86400)).await;

    match check_and_cancel(&order_, &cfg, &max_buffer).await {
        Ok(cancelled) => {
            if !cancelled {
                println!("Cannot auto-cancel order {} for symbol {}", order_.order_id, order_.symbol);
                Ok(false)
            }
            else {
                println!("Auto-cancelled order {} for symbol {}", order_.order_id, order_.symbol);
                Ok(true)
            }
        },
        Err(e) => return Err(format!(
            "Cannot cancnel order {} for {}: {}",
            order_.order_id,
            order_.symbol, e)
            .into()
        )
    }
}

// Strategy
async fn get_historical_data(cfg: &Config, query: &KlineQuery)
-> Result<Vec<CandleSticks>, anyhow::Error>
{
    let timestamp = get_timestamp();
    let mut params = HashMap::new();
    params.insert("symbol", query.symbol.clone());
    params.insert("from_time", query.from_time.to_string());
    params.insert("to_time", query.to_time.to_string());
    params.insert("limit", query.limit.map(|l| l.to_string()).unwrap_or(0.to_string()));

    let client = Client::new();
    let query_string = serde_urlencoded::to_string(params).unwrap();
    let url = format!(
        "{}/api/v1/?kline&query={}timestamp={}", 
        cfg.base_url,
        query_string,
        timestamp
    );

    let response = client.get(&url).send().await?;
    let res_status = response.status();

    if !res_status.is_success() {
        return Err(anyhow::anyhow!(format!("Invalid response received: {}", response.text().await?)));
    }

    let response_json = response.json::<CandleSticks>().await?;
    let raw_data = serde_json::to_value(&response_json).unwrap();
    let data_array = vec![raw_data];
    let mut klines = Vec::new();

    for item in data_array.iter().rev() {
        let arr = item.as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid kline format!"))?;

        if arr.len() < 12 {
            return Err(anyhow::anyhow!("Insufficent kline data fields received"));
        }

        let kline = CandleSticks {
            symbol: query.symbol.clone(),
            timestamp: arr[0].as_i64().unwrap_or(0),
            open: arr[1].as_str().unwrap_or("0").parse::<f64>()?,
            high: arr[2].as_str().unwrap_or("0").parse::<f64>()?,
            low: arr[3].as_str().unwrap_or("0").parse::<f64>()?,
            close: arr[4].as_str().unwrap_or("0").parse::<f64>()?,
            volume: arr[5].as_str().unwrap_or("0").parse::<f64>()?,
            quote_volume: arr[6].as_str().unwrap_or("0").parse::<f64>()?,
            trades_count: Some(arr[7].as_i64().unwrap_or(0) as i32),
            taker_buy_quote_volume: Some(arr[8].as_str().unwrap_or("0").parse::<f64>()?),
            taker_buy_volume: Some(arr[9].as_str().unwrap_or("0").parse::<f64>()?)
        };

        klines.push(kline);
    }
    Ok(klines)

}

async fn fetch_and_save_historical_data(
    cfg: &Config,
    query: &KlineQuery,
    data_manager: &DataManager,
    timeframe: &str
) -> Result<Vec<CandleSticks>, anyhow::Error>
{
    println!("Fecthing {} data for {} from Kucoin", timeframe, query.symbol);

    let data = get_historical_data(cfg, query).await?;

    if data.is_empty() {
        return Err(anyhow::anyhow!("No data received from Kucoin exchange.."));
    }

    data_manager.save_to_csv(&query.symbol, timeframe, &data);

    println!("Successfully fetched and saved data for {} candles", data.len());
    Ok(data)
}

// Strategy

fn calculate_sma(prices: &Vec<f64>, period: usize) -> Vec<f64> {
    let mut sma_values = Vec::new();

    for i in 0.. (prices.len() - period) {
        let windows = prices[..i + period].iter().sum::<f64>() / period as f64;
        sma_values.push(windows);
    }
    sma_values
}

fn calculate_ema(prices: &Vec<f64>, period: usize) -> Vec<f64> {
    let multiplier = 2.0 / (period + 1) as f64;
    let mut ema_values = Vec::new();
    let first_sma = prices[..period].iter().sum::<f64>() / period as f64;
    ema_values.push(first_sma);

    for i in period.. prices.len() {
        let prev_ema = ema_values.last().unwrap();
        let ema = (prices[i] - prev_ema) * multiplier + prev_ema;
        ema_values.push(ema);
    }
    ema_values
} 

fn calculate_rsi(prices: Vec<f64>, period: usize) -> Vec<f64> {
    let changes: Vec<f64> = (1.. prices.len())
    .map(|i| (prices[i] - prices[i - 1]) as f64)
    .collect();

    let mut gains: Vec<f64> = changes.iter()
    .map(|&change| if change > 0.0 {change} else {0.0})
    .collect();

    let mut losses: Vec<f64> = changes
    .iter().map(|&change| if change > 0.0 {change} else {0.0})
    .collect();

    let mut rsi_values = Vec::new();

    for i in (period - 1).. prices.len() {
        let avg_gain = gains.split_off(i - period + 1).iter().sum::<f64>() / period as f64;
        let avg_loss = losses.split_off(i - period + 1).iter().sum::<f64>() / period as f64;

        if avg_loss == 0.0 {
            let _rsi = 100;
        }
        else {
            let rs = avg_gain / avg_loss;
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            rsi_values.push(rsi);
        }
    }
    rsi_values
}

fn calculate_macd(prices: Vec<f64>, fast_period: usize, slow_period: usize, signal_period: usize) -> BTreeMap<String, Vec<f64>> {
    let fast_ema = calculate_ema(&prices, fast_period);
    let slow_ema = calculate_ema(&prices, slow_period);

    let start_idx = fast_period - slow_period;
    let macd_line: Vec<f64> = (1.. slow_ema.len()).map(|i| fast_ema[i + start_idx] - slow_ema[i]).collect();
    let signal_line = calculate_ema(&prices, signal_period);

    let histogram: Vec<f64> = (1.. signal_line.len())
    .map(|i| macd_line[i + macd_line.len() - signal_line.len()] - signal_line[i])
    .collect();

    let mut map = BTreeMap::new();
    map.insert("macd_line".to_string(), macd_line);
    map.insert("signal".to_string(), signal_line);
    map.insert("histogram".to_string(), histogram);
    map
}

fn set_bollinger_bands(prices: Vec<f64>, period: usize, std_multiplier: f64) -> BTreeMap<String, Vec<f64>> {
    let sma = calculate_sma(&prices, period);
    let upper = Vec::new();
    let lower = Vec::new();
    let mut bands = BTreeMap::new();
    bands.insert("middle".to_string(), sma.clone());
    bands.insert("upper".to_string(), upper);
    bands.insert("lower".to_string(), lower);

    for i in (period - 1).. prices.len() {
        let windows = &prices[.. i + period];
        let std_dev: Vec<f64> = windows.iter().map(|x| (x - sma[i - period + 1]).powi(2) as f64).collect();
        let std_dev_sum = std_dev.iter().sum::<f64>();
        let upper_value = vec![&sma[i - period + 1] + (std_dev_sum * std_multiplier)];
        let lower_value = vec![&sma[i - period + 1] - (std_dev_sum * std_multiplier)];

        bands.get("upper").insert(&upper_value);
        bands.get("lower").insert(&lower_value);
    }
    bands
}

// Risk Management

struct PositionSizer {
    account_balance: f64,
    risk_per_trade: f64
}

impl PositionSizer {
    fn init(account_balance: f64, risk_per_trade: f64) -> Self {
        Self {
            account_balance,
            risk_per_trade
        }
    }

    fn calculate_position_size(&self, entry_price: f64, stop_loss: f64) -> f64 {
        let risk_amount = self.account_balance * self.risk_per_trade;
        let stop_distance = (entry_price - stop_loss).abs();

        if stop_distance == 0.0 {
            return 0.0;
        }

        let position_size = risk_amount / stop_distance;

        return position_size.min(self.account_balance * 0.1);
    }
}

struct PortfolioRiskManager {
    max_portfolio_risk: f64,
    max_drawdown: f64,
    peak_balance: f64
}

impl PortfolioRiskManager {
    fn init(max_portfolio_risk: f64, max_drawdown: f64, peak_balance: f64) -> Self {
        Self {
            max_portfolio_risk,
            max_drawdown,
            peak_balance
        }
    }

    fn check_portfolio_risk(&self, positions: &[OrderPosition], account_balance: f64) -> bool {
        let margin: Vec<f64> = positions.iter().map(|pos| pos.margin).collect();
        let total_margin = margin.iter().sum::<f64>();
        let portfolio_risk = total_margin / account_balance;
        return portfolio_risk <= self.max_portfolio_risk;
    }

    fn check_drawdown(&mut self, current_balance: f64) -> bool {
        self.peak_balance = self.peak_balance.max(current_balance);

        if self.peak_balance == 0.0 {
            return true;
        }

        let drawdown = (self.peak_balance - current_balance) / self.peak_balance;
        return drawdown <= self.max_drawdown;
    }
}

// Trading Stategy Framework

trait TradingStrategy {
    fn new(&self, symbol: String, timeframe: String) -> Self where Self: Sized;
    fn analyze_market(&mut self, candles: &mut [CandleSticks]);
    fn should_enter_short(&self) -> bool;
    fn should_enter_long(&self) -> bool;
    fn should_exit_position(&self, positions: &OrderPosition) -> bool;
    fn get_stop_loss(&self, entry_price: f64, side: &Position) -> f64;
    fn get_take_profit(&self, entry_price: f64, side: &Position) -> f64;
    fn indicators_ready(&self) -> bool;
}

struct BaseStrategy {
    symbol: String,
    timeframe: String,
    indicators: HashMap<String, Vec<f64>>
}

impl BaseStrategy {
    fn new(symbol: String, timeframe: String) -> Self {
        Self {
            symbol,
            timeframe,
            indicators: HashMap::new()
        }
    }
}

struct MyStrategy {
    strategy: BaseStrategy
}

impl TradingStrategy for MyStrategy {
    fn new(&self, symbol: String, timeframe: String) -> Self {
        Self {
            strategy: BaseStrategy::new(symbol, timeframe)
        }
    }

    fn analyze_market(&mut self, candles: &mut [CandleSticks]) {
        if let Some(last) = candles.last() {
            self.strategy.indicators.insert("closing_price".into(), vec![last.close]);
        }
    }

    fn get_stop_loss(&self, entry_price: f64, side: &Position) -> f64 {
        match side {
            Position::LONG => entry_price * 0.99,
            Position::SHORT => entry_price * 1.01
        }
    }

    fn get_take_profit(&self, entry_price: f64, side: &Position) -> f64 {
        match side {
            Position::LONG => entry_price * 1.02,
            Position::SHORT => entry_price * 0.98
        }
    }

    fn should_enter_long(&self) -> bool {
        if let Some(last) = self.strategy.indicators.get("closing_price").cloned() {
            last.last() > Some(&100.0)
        }
        else {
            false
        }
    }

    fn should_enter_short(&self) -> bool {
        false
    }

    fn should_exit_position(&self, positions: &OrderPosition) -> bool {
        match positions.position_side {
            Position::LONG => {
                if let Some(last) = self.strategy.indicators.get("closing_price").cloned() {
                    last.last() > Some(&(positions.entry_price + 1.0))
                }
                else {
                    false
                }
            }
            Position::SHORT => false
        }
    }

    fn indicators_ready(&self) -> bool {
        false
    }
}

struct MACStrategy {
    fast_period: usize,
    slow_period: usize,
    rsi_period: usize,
    rsi_overbought: usize,
    rsi_oversold: usize,
    strategy: BaseStrategy
}

impl TradingStrategy for MACStrategy {
    fn new(&self, symbol: String, timeframe: String) -> Self
    {
        Self {
            fast_period: self.fast_period,
            slow_period: self.slow_period,
            rsi_period: self.rsi_period,
            rsi_overbought: 70,
            rsi_oversold: 30,
            strategy: BaseStrategy { symbol, timeframe, indicators: HashMap::new() }
        }
    }

    fn analyze_market(&mut self, candles: &mut [CandleSticks]) {
        if candles.len() < self.slow_period + self.rsi_period { }

        let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();
        self.strategy.indicators = HashMap::new();
        self.strategy.indicators.insert("slow_ma".into(), calculate_ema(&closes, self.slow_period));
        self.strategy.indicators.insert("fast_ma".into(), calculate_ema(&closes, self.fast_period));
        self.strategy.indicators.insert("rsi".into(), calculate_ema(&closes, self.rsi_period));
    }

    fn should_enter_long(&self) -> bool {
        if !self.indicators_ready() {
            return false;
        }
        let slow_ma = self.strategy.indicators.get("slow_ma").cloned().unwrap();
        let fast_ma = self.strategy.indicators.get("fast_ma").cloned().unwrap();
        let rsi = self.strategy.indicators.get("rsi").cloned().unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 && rsi.len() >= 1 {
            let current_golden_cross = fast_ma[fast_ma.len() - 1] > slow_ma[slow_ma.len() - 1];
            let previous_golden_cross = fast_ma[fast_ma.len() - 2] > slow_ma[slow_ma.len() - 2];
            return current_golden_cross && !previous_golden_cross && rsi[rsi.len() - 1] < self.rsi_overbought as f64;
        }

        false
    }

    fn should_enter_short(&self) -> bool {
        if !self.indicators_ready() {
            return false;
        }

        let slow_ma = self.strategy.indicators.get("slow_ma").cloned().unwrap();
        let fast_ma = self.strategy.indicators.get("fast_ma").cloned().unwrap();
        let rsi = self.strategy.indicators.get("rsi").cloned().unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 && rsi.len() >= 1 {
            let current_golden_cross = fast_ma[fast_ma.len() - 1] < slow_ma[slow_ma.len() - 1];
            let previous_golden_cross = fast_ma[fast_ma.len() - 2] < slow_ma[slow_ma.len() - 2];
            return current_golden_cross && !previous_golden_cross && rsi[rsi.len() - 1] > self.rsi_overbought as f64;
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

        let slow_ma = self.strategy.indicators.get("slow_ma").cloned().unwrap();
        let fast_ma = self.strategy.indicators.get("fast_ma").cloned().unwrap();

        if fast_ma.len() >= 2 && slow_ma.len() >= 2 {
            if matches!(positions.position_side, Position::LONG) {
                return slow_ma[slow_ma.len() - 1] < fast_ma[fast_ma.len() - 1] && fast_ma[fast_ma.len() - 2] >= slow_ma[slow_ma.len() - 2];
            }
            else {
                return slow_ma[slow_ma.len() - 1] > fast_ma[fast_ma.len() - 1] && fast_ma[fast_ma.len() - 2] <= slow_ma[slow_ma.len() - 2];
            }
        }
        return false;
    }

    fn indicators_ready(&self) -> bool {
        let required_indicators = ["fast_ma", "slow_ma", "rsi"];
        
        for indicator in required_indicators {
            self.strategy.indicators[indicator].len() > 0;
        }
        return false;
    }
    
}