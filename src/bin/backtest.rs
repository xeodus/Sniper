use std::collections::VecDeque;
use sniper_bot::backtesting::run_backtest;
use sniper_bot::{risk_manager::{AccountState, RiskConfig}, strategy::TradeState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut trade = TradeState {
        order_book_depth: 10,
        imbalance_threshold: 0.20,
        entry_price: 75000.0,
        ema_period: 20,
        ema_value: 0.0,
        ema_count: 0,
        price_buffer: VecDeque::with_capacity(20),
        max_position: 100.0,
        stop_loss: 0.01
    };

    let mut state = AccountState {
        current_position: 50.0,
        max_position: trade.max_position,
        price: 76000.0,
        last_price: 80000.0,
        account_balance: 50000.0,
        unrealised_pnl: 1.24
    };

    let mut risk_cfg = RiskConfig {
        max_drawdown_pct: 0.20,
        max_position_pct: 0.02,
        warn_position_pct: 0.04,
        max_potential_loss: 0.02
    };
    run_backtest("historical_data/market.csv", &mut state, &mut trade, &mut risk_cfg)?;
    Ok(())
}
