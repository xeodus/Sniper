use clap::{arg, Parser};
use dotenv::dotenv;
use anyhow::Result;

use crate::{
    config::AppConfig, data::{Exchange, Trend}, trading_engine::TradingEngine
};

mod data;
mod exchange;
mod store;
mod indicator;
mod strategy;
mod websocket;
mod config;
mod trading_engine;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value="ETH-USDT")]
    symbol: String,
    #[arg(long, default_value="1m")]
    timeframe: String,
    #[arg(long, default_value="orders.db")]
    db: String,
    #[arg(long, default_value="binance")]
    exchange: String,
    #[arg(long, default_value="0.001")]
    quantity: f64,
    #[arg(long, default_value="sidechop")]
    trend: String,
    #[arg(long, default_value="0.01")]
    grid_spacing: f64,
    #[arg(long, default_value="10")]
    grid_levels: usize,
}


#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    // Load environment variables
    dotenv().ok();
    
    let args = Args::parse();
    log::info!("Starting grid bot for {} @ {} on {}", args.symbol, args.timeframe, args.exchange);

    // Load configuration
    let mut config = AppConfig::from_file(&args.db)
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;


    // Override config with command line arguments
    config.trading.symbol = args.symbol.clone();
    config.trading.timeframe = args.timeframe.clone();
    config.trading.quantity = args.quantity;
    config.trading.grid_spacing = args.grid_spacing;
    config.trading.grid_levels = args.grid_levels;
    config.database.path = args.db.clone();

    // Determine exchange type
    let exchange_type = match args.exchange.to_lowercase().as_str() {
        "binance" => Exchange::Binance,
        "kucoin" => Exchange::KuCoin,
        _ => return Err(anyhow::anyhow!("Unsupported exchange: {}", args.exchange)),
    };

    let trend = match args.trend.to_lowercase().as_str() {
        "sidechop" => Trend::SideChop,
        "uptrend" => Trend::UpTrend,
        "downtrend" => Trend::DownTrend,
        &_ => return Err(anyhow::anyhow!("Invaild trend received: {}", args.trend))
    };

    // Create and start trading engine
    let engine = TradingEngine::new(config, exchange_type, trend).await
        .map_err(|e| anyhow::anyhow!("Failed to create trading engine: {}", e))?;

    // Start the trading engine
    engine.start().await
        .map_err(|e| anyhow::anyhow!("Trading engine failed: {}", e))?;

    Ok(())
}
