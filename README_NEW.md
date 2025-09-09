# Sniper Bot - High-Frequency Trading Bot

A sophisticated, low-latency trading bot designed for cryptocurrency exchanges with advanced grid trading strategies and risk management protocols.

## 🚀 Features

- **Advanced Grid Trading**: Automated grid trading with dynamic level adjustment
- **Multi-Exchange Support**: Binance and KuCoin integration
- **Real-time WebSocket**: Live market data and order updates
- **Risk Management**: Built-in position sizing and risk controls
- **Technical Indicators**: SMA, EMA, RSI, MACD, Bollinger Bands, ATR
- **Database Persistence**: SQLite-based order tracking and recovery
- **Async Architecture**: High-performance concurrent processing
- **Comprehensive Error Handling**: Robust error recovery and logging
- **Configuration Management**: Flexible environment-based configuration

## 📋 Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- API keys from supported exchanges (Binance/KuCoin)
- Basic understanding of trading concepts

## 🛠️ Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/xeodus/Sniper.git
   cd Sniper
   ```

2. **Install dependencies**
   ```bash
   cargo build --release
   ```

3. **Set up environment variables**
   ```bash
   cp .env.example .env
   # Edit .env with your API credentials
   ```

## ⚙️ Configuration

### Environment Variables

Create a `.env` file in the project root with the following variables:

```bash
# Exchange API Configuration
BINANCE_API_KEY=your_binance_api_key_here
BINANCE_SECRET_KEY=your_binance_secret_key_here

KUCOIN_API_KEY=your_kucoin_api_key_here
KUCOIN_SECRET_KEY=your_kucoin_secret_key_here
KUCOIN_PASSPHRASE=your_kucoin_passphrase_here

# Trading Configuration
TRADING_SYMBOL=ETH-USDT
TRADING_TIMEFRAME=1m
TRADING_QUANTITY=0.001
TRADING_GRID_SPACING=0.01
TRADING_GRID_LEVELS=10
TRADING_MAX_POSITION=1.0
TRADING_RISK_PERCENTAGE=2.0

# Database Configuration
DATABASE_PATH=orders.db

# Logging Configuration
LOG_LEVEL=info
PAPER_TRADING=true
DEBUG_MODE=false
```

### Command Line Options

```bash
cargo run -- --help
```

Available options:
- `--symbol`: Trading pair (default: ETH-USDT)
- `--timeframe`: Timeframe (default: 1m)
- `--exchange`: Exchange (binance/kucoin)
- `--quantity`: Order quantity (default: 0.001)
- `--grid-spacing`: Grid spacing percentage (default: 0.01)
- `--grid-levels`: Number of grid levels (default: 10)
- `--db`: Database file path (default: orders.db)

## 🚀 Usage

### Basic Usage

```bash
# Run with default settings
cargo run

# Run with custom parameters
cargo run -- --symbol BTC-USDT --timeframe 5m --exchange binance --quantity 0.01

# Run in paper trading mode (recommended for testing)
PAPER_TRADING=true cargo run
```

### Production Usage

```bash
# Build optimized release
cargo build --release

# Run production build
./target/release/sniper_bot --symbol ETH-USDT --exchange binance
```

## 📊 Trading Strategy

### Grid Trading Algorithm

The bot implements a sophisticated grid trading strategy:

1. **Trend Detection**: Uses EMA crossover and ATR for trend identification
2. **Grid Initialization**: Creates buy/sell levels based on market conditions
3. **Dynamic Adjustment**: Adjusts grid based on trend changes
4. **Risk Management**: Implements position sizing and stop-loss mechanisms

### Technical Indicators

- **SMA/EMA**: Moving averages for trend identification
- **RSI**: Relative Strength Index for overbought/oversold conditions
- **MACD**: Moving Average Convergence Divergence for momentum
- **Bollinger Bands**: Volatility-based support/resistance levels
- **ATR**: Average True Range for volatility measurement

## 🏗️ Architecture

### Core Components

```
src/
├── main.rs              # Application entry point
├── trading_engine.rs    # Main trading engine
├── config.rs           # Configuration management
├── error.rs            # Error handling
├── data.rs             # Data structures
├── exchange/           # Exchange integrations
│   ├── binance_auth.rs
│   ├── kucoin_auth.rs
│   └── config.rs
├── websocket/          # WebSocket clients
│   ├── binance_ws.rs
│   ├── kucoin_ws.rs
│   └── ws_client.rs
├── strategy/           # Trading strategies
│   ├── grid_strategy.rs
│   └── trade_strategy.rs
├── indicators.rs       # Technical indicators
└── store.rs           # Database operations
```

### Key Features

- **Async/Await**: Non-blocking I/O for high performance
- **Channel-based Communication**: Efficient inter-component messaging
- **State Management**: Centralized bot state with watch channels
- **Error Recovery**: Comprehensive error handling with retry logic
- **Database Persistence**: Order tracking and recovery

## 🔧 Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test indicators

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for security issues
cargo audit
```

## 📈 Performance

### Optimizations

- **Zero-copy operations** where possible
- **Efficient data structures** (VecDeque for candles)
- **Connection pooling** for HTTP requests
- **Batch database operations**
- **Memory-efficient WebSocket handling**

### Monitoring

The bot provides comprehensive logging:

```bash
# Set log level
RUST_LOG=debug cargo run

# Log to file
LOG_FILE=logs/bot.log cargo run
```

## ⚠️ Risk Disclaimer

**IMPORTANT**: This software is for educational and research purposes. Trading cryptocurrencies involves substantial risk of loss. The authors are not responsible for any financial losses incurred through the use of this software.

### Risk Management Features

- **Paper Trading Mode**: Test strategies without real money
- **Position Sizing**: Automatic position size calculation
- **Stop Loss**: Built-in risk controls
- **Daily Limits**: Maximum daily loss protection
- **Rate Limiting**: Exchange API rate limit compliance

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions
- Add tests for new features
- Update documentation
- Ensure all tests pass
- Use meaningful commit messages

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Issues**: [GitHub Issues](https://github.com/xeodus/Sniper/issues)
- **Discussions**: [GitHub Discussions](https://github.com/xeodus/Sniper/discussions)
- **Documentation**: [Wiki](https://github.com/xeodus/Sniper/wiki)

## 🔄 Changelog

### v2.0.0 (Current)
- Complete architecture rewrite
- Improved error handling
- Enhanced configuration management
- Better WebSocket implementation
- Comprehensive testing suite

### v1.0.0 (Legacy)
- Initial implementation
- Basic grid trading
- Binance/KuCoin support

---

**Happy Trading! 🚀**

*Remember: Always test with paper trading first and never risk more than you can afford to lose.*