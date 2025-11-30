use crate::{
    data::{Candles, OrderReq, OrderType, Position, PositionSide, Side, Signal, TradingBot},
    db::Database,
    position_manager::PositionManager,
    rest_client::BinanceClient,
    signal::MarketSignal,
};
use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

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
            analyzer: Arc::new(RwLock::new(MarketSignal::new())),
            position_manager,
            signal_tx,
            order_tx,
            binance_client,
            account_balance: Arc::new(RwLock::new(initial_balance)),
            db,
        })
    }

    pub async fn initializer(&self) -> Result<()> {
        self.position_manager.load_open_orders().await?;
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
                if let Some(position) = self
                    .position_manager
                    .get_positions_by_id(&position_id)
                    .await
                {
                    let exit_side = match position_side {
                        PositionSide::Long => Side::Sell,
                        PositionSide::Short => Side::Buy,
                    };

                    let req = OrderReq {
                        id: position_id.to_string(),
                        symbol: symbol.to_string(),
                        side: exit_side,
                        price: current_price,
                        size: position.size,
                        order_type: OrderType::Limit,
                        sl: None,
                        tp: None,
                        manual: false,
                    };

                    match self.execute_order(req).await {
                        Ok(_) => {
                            info!("Order succeeded, closing position...");
                            self.position_manager
                                .close_positions(&position_id, current_price)
                                .await?;
                        }
                        Err(e) => {
                            error!("Failed to place order: {}", e);
                        }
                    }
                }

                let analyzer = self.analyzer.read().await;
                let signal_opt = analyzer.analyze(symbol.to_string());

                if let Some(signal) = signal_opt {
                    if let Err(e) = self.db.save_signal(signal.clone()).await {
                        warn!("Failed to save signal onto database: {}", e);
                    }

                    if let Err(e) = self.signal_tx.send(signal.clone()).await {
                        warn!("Failed to send order: {}", e)
                    }

                    let confidence_threahold = Decimal::new(70, 2);

                    if signal.confidence >= confidence_threahold {
                        match signal.action {
                            Side::Buy => {
                                if let Err(e) = self
                                    .execute_entry_order(signal, position_side, OrderType::Market)
                                    .await
                                {
                                    error!("Failed to place buy order for market price: {}", e);
                                }
                            }
                            Side::Sell => {
                                if let Err(e) = self
                                    .execute_entry_order(signal, position_side, OrderType::Market)
                                    .await
                                {
                                    error!("Failed to place sell order for market price: {}", e);
                                }
                            }
                            Side::Hold => {
                                info!(
                                    "Unclear trend detected, so holding the positions for now..."
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /*pub async fn place_manual_order(&self, order: OrderReq) -> Result<()> {
        let mut manual_order = order;
        manual_order.manual = true;
        self.order_tx.send(manual_order).await?;
        info!("Placed manual order!");
        Ok(())
    }*/

    pub async fn execute_entry_order(
        &self,
        signal: Signal,
        position_side: PositionSide,
        order_type: OrderType,
    ) -> Result<()> {
        let account_balance = *self.account_balance.read().await;

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

        if position_size <= Decimal::ZERO {
            self.binance_client.cancel_orders(&order).await?;
            error!("Invalid position size, cancelling the order...");
        }

        if order.tp.is_none() || order.sl.is_none() {
            self.binance_client.cancel_orders(&order).await?;
            error!("Take profit and stop loss is not set, cancelling the order...");
        }

        match self.execute_order(order).await {
            Ok(_) => {
                self.position_manager.open_position(position, false).await?;
                info!("Position opened successfully!");
            }
            Err(e) => {
                warn!("Failed to execute order: {}", e);
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
}
