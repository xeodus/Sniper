use crate::{
    data::{Position, PositionSide},
    db::Database,
};
use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct PositionManager {
    pub position: Arc<RwLock<Vec<Position>>>,
    pub risk_per_trade: Decimal,
    pub db: Arc<Database>,
}

impl PositionManager {
    pub fn new(risk_per_trade: Decimal, db: Arc<Database>) -> Self {
        Self {
            position: Arc::new(RwLock::new(Vec::new())),
            risk_per_trade,
            db,
        }
    }

    pub async fn load_open_orders(&self) -> Result<()> { 
        let positions = self.db.get_open_orders().await?;
        let count = positions.len();
        let mut position = self.position.write().await;
        *position = positions;
        
        info!("Loaded open positions into the database: {}", count);

        Ok(())
    }

    /*pub async fn get_orders(&self) -> Vec<Position> {
        let positions = self.position.read().await;
        positions.clone()
    }*/

    pub async fn get_positions_by_id(&self, position_id: &str) -> Option<Position> {
        let positions = self.position.read().await;

        if let Some(position) = positions.iter().find(|f| f.id == position_id) {
            Some(position.clone())
        }
        else {
            info!("Can't fetch position via ID...");
            None
        }
    }

    pub async fn has_positions(&self) -> bool {
        let positions = self.position.read().await;
        !positions.is_empty()
    }

    pub async fn open_position(&self, position: Position, manual: bool) -> Result<()> {
        if position.entry_price == Decimal::ZERO || position.size == Decimal::ZERO {
            info!("Attempt to open position with size zero, rejected...");
            return Ok(());
        }

        self.db.save_order(&position, manual).await?;

        let mut positions = self.position.write().await;
        positions.push(position.clone());

        info!("New position opened!");
        Ok(())
    }

    pub async fn close_positions(&self, position_id: &str, exit_price: Decimal) -> Result<()> {
        let mut positions = self.position.write().await;

        if !self.has_positions().await {
            return Err(anyhow!("No open positions found to be closed!"));
        }

        if let Some(pos) = positions.iter().find(|p| p.id == position_id) {
            let pnl = match pos.position_side {
                PositionSide::Long => (exit_price - pos.entry_price) * pos.size,
                PositionSide::Short => (pos.entry_price - exit_price) * pos.size,
            };
            self.db.close_order(position_id, exit_price, pnl).await?;
            info!(
                "Closed position for id: {} at price: {} at pnl: {}",
                position_id, exit_price, pnl
            );
        }

        positions.retain(|p| p.id != position_id);

        Ok(())
    }

    pub async fn check_positions(
        &self,
        current_price: Decimal,
        symbol: &str,
    ) -> Vec<(String, Decimal, PositionSide)> {
        let positions = self.position.read().await;
        let mut to_close = Vec::new();

        for position in positions.iter() {
            if position.symbol != symbol {
                continue;
            }

            match position.position_side {
                PositionSide::Long => {
                    if current_price <= position.stop_loss {
                        to_close.push((
                            position.id.clone(),
                            current_price,
                            position.position_side.clone(),
                        ));

                        info!(
                            "Stop loss triggered for Long position for  id: {} at price: {}",
                            position.id, current_price
                        );
                    } else if current_price >= position.take_profit {
                        to_close.push((
                            position.id.clone(),
                            current_price,
                            position.position_side.clone(),
                        ));

                        info!(
                            "Take profit triggered for Long position for id: {} at price: {}",
                            position.id, current_price
                        );
                    }
                }
                PositionSide::Short => {
                    if current_price >= position.stop_loss {
                        to_close.push((
                            position.id.clone(),
                            current_price,
                            position.position_side.clone(),
                        ));

                        info!(
                            "Stop loss triggered for Short position for id: {} at price: {}",
                            position.id, current_price
                        );
                    } else if current_price <= position.take_profit {
                        to_close.push((
                            position.id.clone(),
                            current_price,
                            position.position_side.clone(),
                        ));

                        info!(
                            "Take profit triggered for Short position for id: {} at price: {}",
                            position.id, current_price
                        );
                    }
                }
            }
        }

        to_close
    }

    pub async fn calculate_position_size(
        &self,
        account_balance: Decimal,
        entry_price: Decimal,
        stop_loss: Decimal,
    ) -> Decimal {
        let risk_amount = account_balance * self.risk_per_trade;
        let risk_per_unit = (entry_price - stop_loss).abs();

        if risk_per_unit == Decimal::ZERO {
            return Decimal::ZERO;
        }

        risk_amount / risk_per_unit
    }
}
