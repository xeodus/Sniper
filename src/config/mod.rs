use serde::Deserialize;
use std::{collections::HashMap, env, fs};
use serde_json::Value;
use anyhow::{Context, Ok, Result};
use dotenv::dotenv;

#[derive(Debug, Deserialize)]
struct ExchangeConfig {
    api_key: String,
    secret_key: String,
    base_url: String,
    rate_limiter: RateLimits
}

#[derive(Debug, Deserialize)]
struct RateLimits {
    requests_per_minute: u32,
    brust_limit: u32
}

#[derive(Debug, Deserialize)]
pub struct TradingConfig {
    symbols: Vec<String>,
    base_currency: String,
    quote_currency: String,
    timeframe: String,
    max_position: usize,
    max_position_size: f64,
    max_drawdown: f64
}

#[derive(Debug, Deserialize)]
struct StrategyConfig {
    name: String,
    parameters: HashMap<String, Value>
}

#[derive(Debug, Deserialize)]
struct RiskConfig {
    max_risk_per_trade: f64,
    stop_loss_percentage: f64,
    take_profit_percentage: f64,
    max_daily_loss: f64
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    exchange: ExchangeConfig,
    trading: TradingConfig,
    risk: RiskConfig,
    strategy: StrategyConfig,
    log_level: String
}

fn load_config (path: &str) -> Result<AppConfig> {
    dotenv().context("Failed to upload the dotenv file.");
    let content = fs::read_to_string(path).context("Failed to read the config file.")?;
    let cfg_file = toml::from_str::<AppConfig>(&content).context("Invalid TOML file.")?;
    let exchange = ExchangeConfig {
        api_key: env::var("API_KEY").expect("Missing API key!"),
        secret_key: env::var("SECRET_KEY").expect("Missing secret key!"),
        base_url: env::var("BASE_URL").expect("Missing base url!"),
        rate_limiter: cfg_file.exchange.rate_limiter
    };

    Ok(AppConfig {
        exchange: exchange,
        trading: cfg_file.trading,
        risk: cfg_file.risk,
        strategy: cfg_file.strategy,
        log_level: cfg_file.log_level })
}