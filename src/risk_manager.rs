use crate::execution::Side;

pub struct OrderRequest {
    pub entry_price: f64,
    pub quantity: f64,
    pub stop_loss: f64,
    pub side: Side
}

pub struct AccountState {
    pub current_position: f64,
    pub max_position: f64,
    pub last_price: f64,
    pub entry_price: f64,
    pub account_balance: f64,
    pub unrealised_pnl: f64
}

pub struct RiskConfig {
    pub max_position_pct: f64,
    pub warn_position_pct: f64,
    pub max_drawdown_pct: f64,
    pub max_potential_loss: f64
}

pub trait RiskManager {
    /*fn calculate_position_size(&self, price: f64, risk_pct: f64) -> f64;
    fn update_position(&mut self, filled_quantity: f64, filled_price: f64);*/
    fn check_stop_loss(&mut self, current_price: f64, stop_loss_pct: f64) -> bool;
    fn update_unrealised_pnl(&mut self, current_price: f64);
}

impl RiskManager for AccountState {

    fn check_stop_loss(&mut self, current_price: f64, stop_loss_pct: f64) -> bool {
        if self.current_position > 0.0 {
            let stop_price = self.entry_price * (1.0 - stop_loss_pct);
            let _ = current_price <= stop_price;
            true
        }
        else if self.current_position < 0.0 {
            let stop_price = self.entry_price * (1.0 + stop_loss_pct);
            let _ = current_price >= stop_price;
            true
        }
        else {
            false
        }
    }

    fn update_unrealised_pnl(&mut self, current_price: f64) {
        self.last_price = current_price;
        if self.current_position != 0.0 {
            let price_differ = current_price - self.entry_price;
            self.unrealised_pnl = price_differ * self.current_position;
        }
        else {
            self.unrealised_pnl = 0.0;
        }
    }
}
    