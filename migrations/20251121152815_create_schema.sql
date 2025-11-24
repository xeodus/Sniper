CREATE TABLE IF NOT EXISTS trades (
    trade_id TEXT NOT NULL PRIMARY KEY,
    symbol VARCHAR(50) NOT NULL,
    side TEXT NOT NULL, 
    entry_price DECIMAL(20, 8) NOT NULL,
    quantity DECIMAL(20, 8) NOT NULL,
    stop_loss DECIMAL(20, 8),
    take_profit DECIMAL(20, 8),
    opened_at TIMESTAMPTZ NOT NULL,
    closed_at TIMESTAMPTZ NOT NULL,
    exit_price DECIMAL(20, 8),
    pnl DECIMAL(20, 8),
    status VARCHAR(20) NOT NULL,
    manual BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS signals (
    id TEXT NOT NULL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    action TEXT NOT NULL,
    price DECIMAL(20, 8) NOT NULL,
    confidence DECIMAL(5, 4) NOT NULL,
    trend TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol);
CREATE INDEX IF NOT EXISTS idx_trades_status ON trades(status);
CREATE INDEX IF NOT EXISTS idx_signals_timestamp ON signals(timestamp);
