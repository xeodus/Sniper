use thiserror::Error;

#[derive(Error, Debug)]
pub enum TradingError {
    #[error("Exchange API error: {0}")]
    ExchangeApi(String),
    
    #[error("WebSocket connection error: {0}")]
    WebSocketConnection(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Order placement failed: {0}")]
    OrderPlacement(String),
    
    #[error("Order cancellation failed: {0}")]
    OrderCancellation(String),
    
    #[error("Invalid market data: {0}")]
    InvalidMarketData(String),
    
    #[error("Strategy error: {0}")]
    Strategy(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),
    
    #[error("Invalid quantity: {0}")]
    InvalidQuantity(String),
    
    #[error("Invalid price: {0}")]
    InvalidPrice(String),
}

impl From<reqwest::Error> for TradingError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            TradingError::Network(format!("Request timeout: {}", err))
        } else if err.is_connect() {
            TradingError::Network(format!("Connection error: {}", err))
        } else {
            TradingError::Network(format!("HTTP error: {}", err))
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for TradingError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        TradingError::WebSocketConnection(format!("WebSocket error: {}", err))
    }
}

impl From<rusqlite::Error> for TradingError {
    fn from(err: rusqlite::Error) -> Self {
        TradingError::Database(format!("SQLite error: {}", err))
    }
}

impl From<serde_json::Error> for TradingError {
    fn from(err: serde_json::Error) -> Self {
        TradingError::InvalidMarketData(format!("JSON parsing error: {}", err))
    }
}

pub type TradingResult<T> = Result<T, TradingError>;

/// Error handling utilities
pub struct ErrorHandler;

impl ErrorHandler {
    /// Log error and return a user-friendly message
    pub fn handle_error(error: &TradingError) -> String {
        match error {
            TradingError::ExchangeApi(msg) => {
                log::error!("Exchange API error: {}", msg);
                format!("Exchange API error: {}", msg)
            }
            TradingError::WebSocketConnection(msg) => {
                log::error!("WebSocket connection error: {}", msg);
                format!("WebSocket connection error: {}", msg)
            }
            TradingError::Database(msg) => {
                log::error!("Database error: {}", msg);
                format!("Database error: {}", msg)
            }
            TradingError::Configuration(msg) => {
                log::error!("Configuration error: {}", msg);
                format!("Configuration error: {}", msg)
            }
            TradingError::OrderPlacement(msg) => {
                log::error!("Order placement failed: {}", msg);
                format!("Order placement failed: {}", msg)
            }
            TradingError::OrderCancellation(msg) => {
                log::error!("Order cancellation failed: {}", msg);
                format!("Order cancellation failed: {}", msg)
            }
            TradingError::InvalidMarketData(msg) => {
                log::error!("Invalid market data: {}", msg);
                format!("Invalid market data: {}", msg)
            }
            TradingError::Strategy(msg) => {
                log::error!("Strategy error: {}", msg);
                format!("Strategy error: {}", msg)
            }
            TradingError::Network(msg) => {
                log::error!("Network error: {}", msg);
                format!("Network error: {}", msg)
            }
            TradingError::Authentication(msg) => {
                log::error!("Authentication error: {}", msg);
                format!("Authentication error: {}", msg)
            }
            TradingError::RateLimit(msg) => {
                log::warn!("Rate limit exceeded: {}", msg);
                format!("Rate limit exceeded: {}", msg)
            }
            TradingError::InsufficientBalance(msg) => {
                log::error!("Insufficient balance: {}", msg);
                format!("Insufficient balance: {}", msg)
            }
            TradingError::InvalidSymbol(msg) => {
                log::error!("Invalid symbol: {}", msg);
                format!("Invalid symbol: {}", msg)
            }
            TradingError::InvalidQuantity(msg) => {
                log::error!("Invalid quantity: {}", msg);
                format!("Invalid quantity: {}", msg)
            }
            TradingError::InvalidPrice(msg) => {
                log::error!("Invalid price: {}", msg);
                format!("Invalid price: {}", msg)
            }
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(error: &TradingError) -> bool {
        match error {
            TradingError::Network(_) => true,
            TradingError::WebSocketConnection(_) => true,
            TradingError::RateLimit(_) => true,
            TradingError::ExchangeApi(msg) => {
                // Some exchange API errors are retryable
                msg.contains("timeout") || msg.contains("connection") || msg.contains("rate limit")
            }
            _ => false,
        }
    }

    /// Get retry delay for retryable errors
    pub fn get_retry_delay(error: &TradingError, attempt: u32) -> std::time::Duration {
        match error {
            TradingError::RateLimit(_) => std::time::Duration::from_secs(60), // 1 minute for rate limits
            TradingError::Network(_) | TradingError::WebSocketConnection(_) => {
                // Exponential backoff for network errors
                std::time::Duration::from_secs(2_u64.pow(attempt.min(5)))
            }
            TradingError::ExchangeApi(_) => {
                // Linear backoff for API errors
                std::time::Duration::from_secs(5 * attempt)
            }
            _ => std::time::Duration::from_secs(1),
        }
    }
}