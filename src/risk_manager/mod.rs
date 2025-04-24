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

fn access_risk(account_state: &AccountState, order_request: &OrderRequest) -> RiskCheckResult {

    if order_request.quantity <= 0.0 {
        log::error!("Order quantity must be greater than zero {}%", order_request.quantity);
        return RiskCheckResult::REJECTED
    }
    else if account_state.account_balance <= 0.0 {
        log::error!("Insufficient account balance to place the order {}%", account_state.account_balance);
        return RiskCheckResult::REJECTED
    }
    else {
        println!("Cannot risk check, error state received!");
    }

    let total_potential_position = account_state.current_position + order_request.quantity;

    if total_potential_position > account_state.max_position {
        log::error!("Position size limit exceeded!");
        return RiskCheckResult::REJECTED
    }
    else {
        println!("Cannot risk check, error state received!");
    }

    let position_value = total_potential_position * order_request.entry_price;
    let position_percentage = ((position_value / account_state.account_balance) * 100.0).round() as i64;
    let max_pos_pct_seen = 0;
    let max_position_pct = max(max_pos_pct_seen, position_percentage);

    if position_percentage > max_position_pct {
        log::error!("Position percent {}% exceeds max position percent {}%", position_percentage, max_position_pct);
        return RiskCheckResult::REJECTED
    }
    else {
        println!("Cannot risk check, error state received!");
    }
    
    if order_request.stop_loss != 0.0 {
        let potential_loss_per_unit = order_request.entry_price - order_request.stop_loss;
        let total_potential_loss = potential_loss_per_unit * order_request.quantity;
        let drawdown_pct = ((total_potential_loss / account_state.account_balance) * 100.0).round() as i64;
        let max_drawdown_seen = 0;
        let max_drawdown_pct = max(max_drawdown_seen, drawdown_pct);

        if drawdown_pct > max_drawdown_pct {
            log::error!("Drawdown percentage {}% exceeded maximum allowed drawdown percentage {}%!", drawdown_pct, max_drawdown_pct);
            return RiskCheckResult::REJECTED
        }
    }
    else {
        let default_max_loss_pct = 20.0;
        let potential_loss = (position_value * (default_max_loss_pct * 100.0)).round() as i64;
        let potential_loss_seen = 0;
        let max_potential_loss = max(potential_loss_seen, potential_loss);

        if potential_loss > max_potential_loss {
            log::error!("Potential loss {}% exceeded maximum allowed potential loss {}%", potential_loss, max_potential_loss);
            return RiskCheckResult::REJECTED
        }
    }

    RiskCheckResult::PASSED
}