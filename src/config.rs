use std::env;
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExchangeCfg {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub sandbox: bool,
    pub rate_limit: u32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradingCfg {
    pub symbol: String,
    pub timeframe: String,
    pub quantity: f64,
    pub grid_spacing: f64,
    pub grid_levels: usize,
    pub max_position_size: f64,
    pub risk_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub max_daily_trades: u32,
    pub max_daily_loss: f64
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseCfg {
    pub path: String,
    pub backup_interval: u64,
    pub max_connections: u32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebSocketCfg {
    pub retry_interval: u32,
    pub max_retry_attempts: u32,
    pub max_candles: usize,
    pub heartbeat_interval: u32,
    pub buffer_size: usize
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingCfg {
    pub level: String,
    pub file_path: Option<String>,
    pub max_file_size: u64,
    pub max_files: u32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub exchange: ExchangeCfg,
    pub trading: TradingCfg,
    pub database: DatabaseCfg,
    pub ws: WebSocketCfg,
    pub logging: LoggingCfg,
    pub paper_trading: bool,
    pub debug_mode: bool
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            exchange: ExchangeCfg {
                api_key: String::new(),
                secret_key: String::new(),
                passphrase: String::new(),
                sandbox: true,
                rate_limit: 1200
            },
            trading: TradingCfg {
                symbol: "ETH-USDT".into(),
                timeframe: "1m".into(),
                quantity: 0.01,
                grid_spacing: 0.01,
                grid_levels: 10,
                max_position_size: 1.0,
                risk_pct: 2.0,
                stop_loss_pct: 5.0,
                take_profit_pct: 10.0,
                max_daily_trades: 100,
                max_daily_loss: 50.0
            },
            database: DatabaseCfg {
                path: String::new(),
                backup_interval: 3600,
                max_connections: 100
            },
            ws: WebSocketCfg {
                retry_interval: 5,
                max_retry_attempts: 10,
                max_candles: 20,
                heartbeat_interval: 30,
                buffer_size: 1000
            },
            logging: LoggingCfg {
                level: String::new(),
                file_path: Some(String::new()),
                max_file_size: 10 * 1024 * 1024,
                max_files: 5
            },
            paper_trading: true,
            debug_mode: false
        }
    }
}

impl AppConfig {
    /*pub fn load_env() -> Result<Self> {
        let mut config = Self::default();
        config.exchange.api_key = env::var("API_KEY").expect("API key not found!");
        config.exchange.secret_key = env::var("SECRET_KEY").expect("secret key not found!");
        config.exchange.passphrase = env::var("PASSPHRASE").expect("passphrase not found!");
        config.exchange.sandbox = env::var("SANDBOX")
            .unwrap_or_else(|_| "true".to_string())
            .parse().unwrap();
        config.trading.symbol = env::var("TRADING_SYMBOL").expect("Trading signal not found!");
        config.trading.timeframe = env::var("TIMEFRAME").expect("Timeframe not set");
        config.trading.quantity = env::var("TRADING_QUANTITY").unwrap().parse().unwrap_or(0.0);
        config.trading.grid_spacing = env::var("GRID_SPACING").unwrap().parse().unwrap_or(0.0);
        config.trading.grid_levels = env::var("GRID_LEVELS").unwrap().parse().unwrap_or(0);
        config.trading.max_position_size = env::var("MAX_POSITION_LEVELS").unwrap().parse().unwrap_or(0.0);
        config.trading.risk_pct = env::var("RISK_PCT").unwrap().parse().unwrap_or(0.0);
        config.database.path = env::var("DATABASE_PATH").expect("Database path not set!");
        config.ws.retry_interval = env::var("RETRY_INTERVAL").unwrap().parse().unwrap_or(0);
        config.logging.level = env::var("LOG_LEVEL").expect("Log level not set!");
        config.logging.file_path = Some(env::var("FILE_PATH").expect("File path not set!"));
        config.paper_trading = env::var("PAPER_TRADING").unwrap().parse().unwrap_or(true);
        config.debug_mode = env::var("DEBUG_MODE").unwrap().parse().unwrap_or(true);

        config.validate()?;
        Ok(config)
    }*/

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.exchange.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key not found!"));
        }

        if self.exchange.secret_key.is_empty() {
            return Err(anyhow::anyhow!("secret key is not found!"));
        }

        if self.exchange.passphrase.is_empty() {
            return Err(anyhow::anyhow!("passphrase is not found!"));
        }

        if self.trading.grid_levels == 0 {
            return Err(anyhow::anyhow!("grid levels must be greater than zero!"));
        }

        if self.trading.grid_spacing <= 0.0 || self.trading.grid_spacing >= 1.0 {
            return Err(anyhow::anyhow!("grid spacing must be greater than zero!"));
        }

        if self.trading.risk_pct <= 0.0 {
            return Err(anyhow::anyhow!("risk percentage must be greater than zero!"));
        }

        if self.trading.quantity == 0.0 {
            return Err(anyhow::anyhow!("trading quantity must be greater than zero!"));
        }

        if self.trading.stop_loss_pct <= 0.0 {
            return Err(anyhow::anyhow!("stop loss percentage must be greater than zero!"));
        }

        if self.database.path.is_empty() {
            return Err(anyhow::anyhow!("database path not found!"));
        }

        if self.ws.retry_interval == 0 {
            return Err(anyhow::anyhow!("WebSocket retry must be greater than zero!"));
        }

        let valid_levels = vec!["tracing", "debug", "error", "warn", "info"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(anyhow::anyhow!(format!("Invalid log levels detected: {}", self.logging.level)));
        }

        Ok(())
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| eprintln!("Failed to read from the file: {}", e)).unwrap();
        let config = toml::from_str(content.as_str())
            .map_err(|e| eprintln!("Failed to deserialize the content: {}", e)).unwrap();
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| eprintln!("Failed to serialize config: {}", e)).unwrap();
        std::fs::write(path, content).map_err(|e| eprintln!("Failed to write to config file: {}", e)).unwrap();
        Ok(())
    }

    pub fn get_exchange_config(&self, exchange: &str) -> Result<ExchangeCfg> {
        let mut config = self.exchange.clone();
        match exchange {
            "binance" => {
                config.api_key = env::var("BINANCE_API_KEY").expect("Binance API key not set!");
                config.secret_key = env::var("BINANCE_SECRET_KEY").expect("Binance secret key not set!");
                config.passphrase = env::var("BINANCE_PASSPHRASE").expect("Binance passphrase not set");
                config.sandbox = env::var("SANDBOX").unwrap_or_else(|_| "true".to_string()).parse().unwrap();
                config.rate_limit = env::var("RATE_LIMIT").unwrap().parse().unwrap_or(0);
                Ok(config)
            },
            "kucoin" => {
                config.api_key = env::var("KUCOIN_API_KEY").expect("KuCoin API key not set!");
                config.secret_key = env::var("KUCOIN_SECRET_KEY").expect("KuCoin secret key not set!");
                config.passphrase = env::var("KUCOIN_PASSPHRASE").expect("KuCoin passphrase not set!");
                config.sandbox = env::var("SANDBOX").unwrap().parse().unwrap_or(true);
                config.rate_limit = env::var("RATE_LIMIT").unwrap().parse().unwrap_or(0);
                Ok(config)
            },
            &_ => todo!() 
        }
    }
}
