use crate::{
    data::{Candles, OrderReq, OrderType, Position, PositionSide, Side, Signal, TradingBot},
    db::Database,
    position_manager::PositionManager,
    rest_client::BinanceClient,
    signal::MarketSignal,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;
use uuid::Uuid;

impl TradingBot {
    pub fn new(
        signal_tx: mpsc::Sender<Signal>,
        order_tx: mpsc::Sender<OrderReq>,
        initial_balance: Decimal,
        binance_client: Arc<BinanceClient>,
        db: Arc<Database>,
    ) -> Result<Self> {
        let position_manager = Arc::new(PositionManager::new(Decimal::new(2, 2), db.clone()));
        Ok(Self {
            current: None,
            analyzer: Arc::new(RwLock::new(MarketSignal::new())),
            position_manager,
            signal_tx,
            order_tx,
            binance_client,
            account_balace: Arc::new(RwLock::new(initial_balance)),
            db,
        })
    }

    pub async fn initializer(&self) -> Result<()> {
        self.position_manager.load_open_orders().await?;
        Ok(())
    }

    pub async fn place_manual_order(&self, order: OrderReq) -> Result<()> {
        let mut manual_order = order;
        manual_order.manual = true;
        self.order_tx.send(manual_order).await?;
        Ok(())
    }

    pub async fn execute_entry_order(
        &self,
        signal: Signal,
        position_side: PositionSide,
        order_type: OrderType,
    ) -> Result<()> {
        let account_balance = *self.account_balace.read().await;

        let (take_profit, stop_loss) = match position_side {
            PositionSide::Long => (
                signal.price * Decimal::new(104, 2),
                signal.price * Decimal::new(98, 2),
            ),
            PositionSide::Short => (
                signal.price * Decimal::new(96, 2),
                signal.price * Decimal::new(102, 2),
            ),
        };

        let position_size = self
            .position_manager
            .calculate_position_size(account_balance, signal.price, stop_loss)
            .await;

        if position_size <= Decimal::ZERO {
            return Err(anyhow!("position size can't be zero or less than zero"));
        }

        let order = OrderReq {
            id: signal.id.clone(),
            symbol: signal.symbol.clone(),
            side: signal.action.clone(),
            price: signal.price,
            size: position_size,
            order_type,
            tp: Some(take_profit),
            sl: Some(stop_loss),
            manual: false,
        };

        let position = Position {
            id: signal.id.clone(),
            symbol: signal.symbol.clone(),
            entry_price: signal.price,
            size: position_size,
            position_side,
            opened_at: Utc::now().timestamp(),
            take_profit,
            stop_loss,
        };

        match self.current {
            None => {
                self.place_manual_order(order).await?;
                info!("Placed manual order on the exchange!");
                self.db.save_signal(signal).await?;
                info!("Signal saved into the database!");
                self.db.save_order(&position, true).await?;
                info!("Manual order saved in the database!");
            }
            Some(_) => {
                self.execute_order(order).await?;
                info!("Placed order on Binance...");
                self.db.save_signal(signal).await?;
                info!("Signal saved into the database!");
                self.db.save_order(&position, false).await?;
                info!("Order saved on the database!");
            }
        }

        Ok(())
    }

    pub async fn execute_order(&self, order: OrderReq) -> Result<()> {
        if matches!(order.order_type, OrderType::Limit) {
            self.binance_client.place_limit_order(&order).await?;
            println!("Placed limit order for: {}", order.id);
        } else if matches!(order.order_type, OrderType::Market) {
            self.binance_client.place_market_order(&order).await?;
            println!("Placed market order for: {}", order.id);
        }

        Ok(())
    }

    pub async fn process_candle(&self, candle: Candles, symbol: &str) -> Result<()> {
        {
            let mut analyzer = self.analyzer.write().await;
            analyzer.add_candles(candle.clone());
        }

        let position_to_close = self
            .position_manager
            .check_positions(candle.close, symbol)
            .await;

        if !position_to_close.is_empty() {
            for (position_id, current_price, position_side) in position_to_close {
                let exit_side = match position_side {
                    PositionSide::Long => Side::Sell,
                    PositionSide::Short => Side::Buy,
                };

                if let Ok(position) = self.position_manager.get_orders().await {
                    let exit_order = OrderReq {
                        id: Uuid::new_v4().to_string(),
                        symbol: symbol.to_string(),
                        side: exit_side,
                        order_type: OrderType::Market,
                        size: position.size,
                        price: current_price,
                        sl: None,
                        tp: None,
                        manual: false,
                    };

                    match self.execute_order(exit_order).await {
                        Ok(_) => {
                            self.position_manager
                                .close_positions(&position_id, current_price)
                                .await?;
                            info!("Position closed after placing the order, successfully!");
                        }
                        Err(e) => {
                            info!("Failed to exit position: {:?}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
