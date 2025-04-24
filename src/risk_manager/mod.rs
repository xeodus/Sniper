use std::{cmp::max, collections::BTreeMap};
use crate::execution::Side;

struct OrderRequest {
    entry_price: f64,
    quantity: f64,
    stop_loss: f64,
    side: Side
}

struct AccountState {
    current_position: f64,
    max_position: f64,
    price: f64,
    account_balance: f64,
    unrealised_pnl: f64
}

enum RiskCheckResult {
    REJECTED,
    PASSED,
    WARNING
}

struct RiskConfig {
    max_position_pct: f64,
    warn_position_pct: f64,
    max_drawdown_pct: f64,
    max_potential_loss: f64
}

impl RiskConfig {
    fn check_quantity(&self, req: &OrderRequest) -> RiskCheckResult {
        if req.quantity <= 0.0 {
            log::error!("Order quantity must be greater than zero {}%", req.quantity);
            return RiskCheckResult::REJECTED
        }
        RiskCheckResult::PASSED
    }

    fn check_balance(&mut self, state: &AccountState, req: &OrderRequest) -> RiskCheckResult {
       if state.account_balance <= 0.0 {
        log::error!("Order quantity must be greater than zero {}%", req.quantity);
        return RiskCheckResult::REJECTED
       }
       RiskCheckResult::PASSED
    }

    fn check_position(&mut self, state: &AccountState, req: &OrderRequest) -> RiskCheckResult {
        let total_potential_position = state.current_position + req.quantity;

        if total_potential_position > state.max_position {
            log::error!("Position size limit exceeded!");
            return RiskCheckResult::REJECTED
        }

        let position_value = total_potential_position * req.entry_price;
        let position_percentage = (position_value / state.account_balance) * 100.0;

        if position_percentage > self.max_position_pct {
            log::error!("Position percent {}% exceeds max position percent {}%", position_percentage, self.max_position_pct);
            return RiskCheckResult::REJECTED
        }
        else if position_percentage > self.warn_position_pct {
            log::warn!("Position percentage exceeded safe theshold, warning triggered {}%", position_percentage);
            return RiskCheckResult::WARNING
        }
        RiskCheckResult::PASSED
    }

    fn check_drawdown(&mut self, state: &AccountState, req: &OrderRequest) -> RiskCheckResult {
        if req.stop_loss != 0.0 {
            let potential_loss_per_unit = match req.side {
                Side::BUY => req.entry_price - req.stop_loss,
                Side::SELL => req.stop_loss - req.entry_price
            };
            let total_potential_loss = potential_loss_per_unit * req.quantity;
            let drawdown_pct = (total_potential_loss / state.account_balance) * 100.0;
    
            if drawdown_pct > self.max_drawdown_pct {
                log::error!("Drawdown percentage {}% exceeded maximum allowed drawdown percentage {}%!", drawdown_pct, self.max_drawdown_pct);
                return RiskCheckResult::REJECTED
            }
            RiskCheckResult::PASSED
        }
        else {
            let default_max_loss_pct = 20.0;
            let total_potential_position = state.current_position + req.quantity;
            let position_value = total_potential_position * req.entry_price;
            let potential_loss = position_value * (default_max_loss_pct * 100.0);
            if potential_loss > self.max_potential_loss {
                log::error!("Potential loss {}% exceeded maximum allowed potential loss {}%", potential_loss, self.max_potential_loss);
                return RiskCheckResult::REJECTED
            }
            RiskCheckResult::PASSED
        }
    }
}

