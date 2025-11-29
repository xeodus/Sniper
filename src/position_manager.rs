use std::sync::Arc;
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use anyhow::{anyhow, Result};
use tracing::info;
use crate::{data::{Position, PositionSide}, db::Database};

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
        let position = self.db.get_open_orders().await.unwrap();
        let mut pos = self.position.write().await; 
        *pos = position;
        info!("Loaded open positions into the database: {}", pos.len());
        Ok(())
    } 

    pub async fn get_orders(&self) -> Result<Position> {
        let open_orders = self.db.get_open_orders().await?;

        if open_orders.is_empty() {
            info!("Failed to fetch open order from the database...");
        }

        if let Some(order) = open_orders.into_iter().next() {
            Ok(order)
        }
        else {
            info!("Can't find any open order...");
            Err(anyhow!("No open orders..."))
        }
    } 

    pub async fn close_positions(&self, position_id: &str, exit_price: Decimal) -> Result<()> {
        let mut positions = self.position.write().await;
        
        if let Some(pos) = positions.iter().find(|p| p.id == position_id) {
            let pnl = match pos.position_side {
                PositionSide::Long => {
                    (exit_price - pos.entry_price) * pos.size
                },
                PositionSide::Short => {
                    (pos.entry_price - exit_price) * pos.size
                }
            };
            self.db.close_order(position_id, exit_price, pnl).await?;
            info!("Closed position for id: {} at price: {} at pnl: {}", position_id, exit_price, pnl);
        }
        
        positions.retain(|p| p.id != position_id);

        Ok(())
    }

    pub async fn check_positions(&self, current_price: Decimal, symbol: &str) -> Vec<(String, Decimal, PositionSide)> {
        let positions = self.position.read().await;
        let mut to_close = Vec::new();

        for position in positions.iter() {
            if position.symbol != symbol {
                continue;
            }

            match position.position_side {
                PositionSide::Long => {
                    if current_price <= position.stop_loss {
                        to_close.push(
                            (position.id.clone(), current_price, position.position_side.clone())
                        );

                        info!("Stop loss triggered for Long position for  id: {} at price: {}", position.id, current_price);
                    }
                    else if current_price >= position.take_profit {
                        to_close.push(
                            (position.id.clone(), current_price, position.position_side.clone())
                        );

                        info!("Take profit triggered for Long position for id: {} at price: {}", position.id, current_price);
                    }
                },
                PositionSide::Short => {
                    if current_price >= position.stop_loss {
                        to_close.push(
                            (position.id.clone(), current_price, position.position_side.clone())
                        );

                        info!("Stop loss triggered for Short position for id: {} at price: {}", position.id, current_price);
                    }
                    else if current_price <= position.take_profit {
                        to_close.push(
                            (position.id.clone(), current_price, position.position_side.clone())
                        );

                        info!("Take profit triggered for Short position for id: {} at price: {}", position.id, current_price);
                    }
                }
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
