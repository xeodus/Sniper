use serde::{Deserialize, Serialize};
use std::env;
use anyhow::Result;
use crate::error::{TradingError, TradingResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: Option<String>,
    pub sandbox: bool,
    pub rate_limit: u32, // requests per minute
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub symbol: String,
    pub timeframe: String,
    pub quantity: f64,
    pub grid_spacing: f64,
    pub grid_levels: usize,
    pub max_position_size: f64,
    pub risk_percentage: f64,
    pub stop_loss_percentage: f64,
    pub take_profit_percentage: f64,
    pub max_daily_trades: u32,
    pub max_daily_loss: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub backup_interval: u64, // seconds
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub reconnect_interval: u64, // seconds
    pub max_reconnect_attempts: u32,
    pub heartbeat_interval: u64, // seconds
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
    pub max_file_size: u64, // bytes
    pub max_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub exchange: ExchangeConfig,
    pub trading: TradingConfig,
    pub database: DatabaseConfig,
    pub websocket: WebSocketConfig,
    pub logging: LoggingConfig,
    pub paper_trading: bool,
    pub debug_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            exchange: ExchangeConfig {
                api_key: String::new(),
                secret_key: String::new(),
                passphrase: None,
                sandbox: true,
                rate_limit: 1200, // 20 requests per second
            },
            trading: TradingConfig {
                symbol: "ETH-USDT".to_string(),
                timeframe: "1m".to_string(),
                quantity: 0.001,
                grid_spacing: 0.01,
                grid_levels: 10,
                max_position_size: 1.0,
                risk_percentage: 2.0,
                stop_loss_percentage: 5.0,
                take_profit_percentage: 10.0,
                max_daily_trades: 100,
                max_daily_loss: 100.0,
            },
            database: DatabaseConfig {
                path: "orders.db".to_string(),
                backup_interval: 3600, // 1 hour
                max_connections: 10,
            },
            websocket: WebSocketConfig {
                reconnect_interval: 5,
                max_reconnect_attempts: 10,
                heartbeat_interval: 30,
                buffer_size: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_path: None,
                max_file_size: 10 * 1024 * 1024, // 10MB
                max_files: 5,
            },
            paper_trading: true,
            debug_mode: false,
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> TradingResult<Self> {
        let mut config = Self::default();

        // Exchange configuration
        config.exchange.api_key = env::var("EXCHANGE_API_KEY")
            .map_err(|_| TradingError::Configuration("EXCHANGE_API_KEY not found".to_string()))?;
        
        config.exchange.secret_key = env::var("EXCHANGE_SECRET_KEY")
            .map_err(|_| TradingError::Configuration("EXCHANGE_SECRET_KEY not found".to_string()))?;
        
        config.exchange.passphrase = env::var("EXCHANGE_PASSPHRASE").ok();
        
        config.exchange.sandbox = env::var("EXCHANGE_SANDBOX")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        // Trading configuration
        if let Ok(symbol) = env::var("TRADING_SYMBOL") {
            config.trading.symbol = symbol;
        }
        
        if let Ok(timeframe) = env::var("TRADING_TIMEFRAME") {
            config.trading.timeframe = timeframe;
        }
        
        if let Ok(quantity) = env::var("TRADING_QUANTITY") {
            config.trading.quantity = quantity.parse()
                .map_err(|_| TradingError::Configuration("Invalid TRADING_QUANTITY".to_string()))?;
        }
        
        if let Ok(grid_spacing) = env::var("TRADING_GRID_SPACING") {
            config.trading.grid_spacing = grid_spacing.parse()
                .map_err(|_| TradingError::Configuration("Invalid TRADING_GRID_SPACING".to_string()))?;
        }
        
        if let Ok(grid_levels) = env::var("TRADING_GRID_LEVELS") {
            config.trading.grid_levels = grid_levels.parse()
                .map_err(|_| TradingError::Configuration("Invalid TRADING_GRID_LEVELS".to_string()))?;
        }
        
        if let Ok(max_position) = env::var("TRADING_MAX_POSITION") {
            config.trading.max_position_size = max_position.parse()
                .map_err(|_| TradingError::Configuration("Invalid TRADING_MAX_POSITION".to_string()))?;
        }
        
        if let Ok(risk_pct) = env::var("TRADING_RISK_PERCENTAGE") {
            config.trading.risk_percentage = risk_pct.parse()
                .map_err(|_| TradingError::Configuration("Invalid TRADING_RISK_PERCENTAGE".to_string()))?;
        }

        // Database configuration
        if let Ok(db_path) = env::var("DATABASE_PATH") {
            config.database.path = db_path;
        }

        // WebSocket configuration
        if let Ok(reconnect_interval) = env::var("WS_RECONNECT_INTERVAL") {
            config.websocket.reconnect_interval = reconnect_interval.parse()
                .map_err(|_| TradingError::Configuration("Invalid WS_RECONNECT_INTERVAL".to_string()))?;
        }

        // Logging configuration
        if let Ok(log_level) = env::var("LOG_LEVEL") {
            config.logging.level = log_level;
        }
        
        if let Ok(log_file) = env::var("LOG_FILE") {
            config.logging.file_path = Some(log_file);
        }

        // General configuration
        config.paper_trading = env::var("PAPER_TRADING")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);
        
        config.debug_mode = env::var("DEBUG_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from file
    pub fn from_file(path: &str) -> TradingResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TradingError::Configuration(format!("Failed to read config file: {}", e)))?;
        
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| TradingError::Configuration(format!("Failed to parse config file: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> TradingResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| TradingError::Configuration(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| TradingError::Configuration(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> TradingResult<()> {
        // Validate exchange configuration
        if self.exchange.api_key.is_empty() {
            return Err(TradingError::Configuration("API key cannot be empty".to_string()));
        }
        
        if self.exchange.secret_key.is_empty() {
            return Err(TradingError::Configuration("Secret key cannot be empty".to_string()));
        }

        // Validate trading configuration
        if self.trading.quantity <= 0.0 {
            return Err(TradingError::Configuration("Trading quantity must be positive".to_string()));
        }
        
        if self.trading.grid_spacing <= 0.0 || self.trading.grid_spacing >= 1.0 {
            return Err(TradingError::Configuration("Grid spacing must be between 0 and 1".to_string()));
        }
        
        if self.trading.grid_levels == 0 {
            return Err(TradingError::Configuration("Grid levels must be greater than 0".to_string()));
        }
        
        if self.trading.risk_percentage <= 0.0 || self.trading.risk_percentage > 100.0 {
            return Err(TradingError::Configuration("Risk percentage must be between 0 and 100".to_string()));
        }

        // Validate database configuration
        if self.database.path.is_empty() {
            return Err(TradingError::Configuration("Database path cannot be empty".to_string()));
        }

        // Validate WebSocket configuration
        if self.websocket.reconnect_interval == 0 {
            return Err(TradingError::Configuration("Reconnect interval must be greater than 0".to_string()));
        }

        // Validate logging configuration
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(TradingError::Configuration(format!("Invalid log level: {}", self.logging.level)));
        }

        Ok(())
    }

    /// Get exchange-specific configuration
    pub fn get_exchange_config(&self, exchange: &str) -> TradingResult<ExchangeConfig> {
        match exchange.to_lowercase().as_str() {
            "binance" => {
                let mut config = self.exchange.clone();
                config.api_key = env::var("BINANCE_API_KEY")
                    .or_else(|_| Ok(self.exchange.api_key.clone()))
                    .map_err(|_| TradingError::Configuration("BINANCE_API_KEY not found".to_string()))?;
                config.secret_key = env::var("BINANCE_SECRET_KEY")
                    .or_else(|_| Ok(self.exchange.secret_key.clone()))
                    .map_err(|_| TradingError::Configuration("BINANCE_SECRET_KEY not found".to_string()))?;
                Ok(config)
            }
            "kucoin" => {
                let mut config = self.exchange.clone();
                config.api_key = env::var("KUCOIN_API_KEY")
                    .or_else(|_| Ok(self.exchange.api_key.clone()))
                    .map_err(|_| TradingError::Configuration("KUCOIN_API_KEY not found".to_string()))?;
                config.secret_key = env::var("KUCOIN_SECRET_KEY")
                    .or_else(|_| Ok(self.exchange.secret_key.clone()))
                    .map_err(|_| TradingError::Configuration("KUCOIN_SECRET_KEY not found".to_string()))?;
                config.passphrase = env::var("KUCOIN_PASSPHRASE")
                    .or_else(|_| Ok(self.exchange.passphrase.clone().unwrap_or_default()))
                    .ok();
                Ok(config)
            }
            _ => Err(TradingError::Configuration(format!("Unsupported exchange: {}", exchange))),
        }
    }
}