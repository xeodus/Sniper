// Risk Management

use crate::data::{OrderPosition, PortfolioRiskManager, PositionSizer};

impl PositionSizer {
    pub fn init(account_balance: f64, risk_per_trade: f64) -> Self {
        Self {
            account_balance,
            risk_per_trade,
            max_position_size: account_balance * 0.1
        }
    }

    pub fn calculate_position_size(&self, entry_price: f64, stop_loss: f64) -> f64 {
        let risk_amount = self.account_balance * self.risk_per_trade;
        let stop_distance = (entry_price - stop_loss).abs();

        if stop_distance == 0.0 {
            return 0.0;
        }

        let position_size = risk_amount / stop_distance;

        return position_size;
    }
}

impl PortfolioRiskManager {
    pub fn init(max_portfolio_risk: f64, max_drawdown: f64, peak_balance: f64) -> Self {
        Self {
            max_portfolio_risk,
            max_drawdown,
            peak_balance
        }
    }

    pub fn check_portfolio_risk(&self, positions: &[OrderPosition], account_balance: f64) -> bool {
        let margin: Vec<f64> = positions.into_iter().map(|pos| pos.margin).collect();
        let total_margin = margin.iter().sum::<f64>();
        let portfolio_risk = total_margin / account_balance;
        return portfolio_risk <= self.max_portfolio_risk;
    }

    pub fn check_drawdown(&mut self, current_balance: f64) -> bool {
        self.peak_balance = self.peak_balance.max(current_balance);

        if self.peak_balance == 0.0 {
            return true;
        }

        let drawdown = (self.peak_balance - current_balance) / self.peak_balance;
        return drawdown <= self.max_drawdown;
    }
}