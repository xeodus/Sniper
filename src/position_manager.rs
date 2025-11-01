use std::sync::Arc;
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::info;
use crate::{data::Position, db::Database};

pub struct PositionManager {
    pub position: Arc<RwLock<Vec<Position>>>,
    pub risk_per_trade: Decimal,
    pub db: Arc<Database>
}

impl PositionManager {
    pub fn new(risk_per_trade: Decimal, db: Arc<Database>) -> Self {
        Self {
            position: Arc::new(RwLock::new(Vec::new())),
            risk_per_trade,
            db
        }
    }

    pub async fn load_open_orders(&self) -> Result<()> {
        let position = self.db.get_open_orders().await?;
        let mut pos = self.position.write().await; 
        *pos = position;
        info!("Loaded open positions into the database: {}", pos.len());
        Ok(())
    }

    pub async fn open_positions(&self, position: Position, manual: bool) -> Result<()> {
        self.db.save_order(&position, manual).await?;
        let mut positions = self.position.write().await;
        positions.push(position.clone());
        Ok(())
    }

    pub async fn close_positions(&self, position_id: &str, exit_price: Decimal) -> Result<()> {
        let mut positions = self.position.write().await;

        if let Some(pos) = positions.iter().find(|p| p.id == position_id) {
            let pnl = (exit_price - pos.entry_price) * pos.size;
            self.db.close_order(position_id, exit_price, pnl).await?;
            info!("Position closed: {} for PnL: {}", position_id, pnl);
        }

        positions.retain(|p| p.id != position_id);
        Ok(())
    }

    pub async fn check_positions(&self, current_price: Decimal, symbol: &str) -> Vec<(String, Decimal)> {
        let positions = self.position.read().await;
        let mut to_close = Vec::new();

        for position in positions.iter() {
            if position.symbol != symbol {
                continue;
            }

            if current_price < position.stop_loss {
                info!("Stop loss triggered for id {} at  price: {}", position.id, current_price);
                to_close.push((position.id.clone(), current_price));
            }

            if current_price > position.take_profit {
                info!("Take profit triggered for id {} at price: {}", position.id, current_price);
                to_close.push((position.id.clone(), current_price));
            }
        }

        to_close
    }

    pub async fn calculate_position_size(&self, account_balance: Decimal, entry_price: Decimal, stop_loss: Decimal) -> Decimal {
        let risk_amount = account_balance * self.risk_per_trade;
        let risk_per_unit = (entry_price - stop_loss).abs();

        if risk_per_unit == Decimal::ZERO {
            return Decimal::ZERO;
        }

        risk_amount / risk_per_unit
    }
}
