use std::sync::Arc;
use anyhow::Result;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use crate::{data::{Candles, OrderReq, OrderType, Side, Signal, TradingBot},
    db::Database, position_manager::PositionManager, 
    rest_client::BinanceClient, signal::MarketSignal};

impl TradingBot {
    pub fn new(signal_tx: mpsc::Sender<Signal>, 
        order_tx: mpsc::Sender<OrderReq>, 
        initial_balance: Decimal, 
        binance_client: Arc<BinanceClient>,        
        db: Arc<Database>) -> Result<Self>
    {
        let position_manager = Arc::new(PositionManager::new(Decimal::new(2, 2), db.clone()));
        Ok(Self {
            analyzer: Arc::new(RwLock::new(MarketSignal::new())),
            position_manager,
            signal_tx,
            order_tx,
            binance_client,
            account_balace: Arc::new(RwLock::new(initial_balance)),
            db
        })
    }

    pub async fn initializer(&self) -> Result<()> {
        self.position_manager.load_open_orders().await?;
        Ok(())
    }

    pub async fn process_candle(&self, candle: Candles, symbol: &str) -> Result<()> {
        let position_to_close = self.position_manager.check_positions(candle.close, symbol).await;

        let order = OrderReq {
            symbol: symbol.to_string(),
            id: Uuid::new_v4().to_string(),
            side: Side::Sell,
            order_type: OrderType::Market,
            size: Decimal::ONE,
            price: Decimal::ONE_HUNDRED,
            sl: None,
            tp: None,
            manual: false
        };

        for (id, exit_price) in position_to_close {
            self.position_manager.close_positions(&id, exit_price).await?;
            self.order_tx.send(order.clone()).await?;
        }

        let analyzer = self.analyzer.read().await;
        if let Some(signal) = analyzer.analyze(symbol.to_string()) {
            self.db.save_signal(signal.clone()).await?;
            let decimal_value = Decimal::from_f64(0.7).unwrap();

            if signal.confidence > decimal_value {
                self.order_tx.send(order).await?;

                if signal.action == Side::Buy {
                    self.execute_buy_order(signal).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn execute_buy_order(&self, signal: Signal) -> Result<()> {
        let account_balance = self.account_balace.read().await;
        let stop_loss = signal.price * Decimal::new(98, 2);
        let take_profit = signal.price * Decimal::new(104, 2);

        let position_size = self.position_manager
            .calculate_position_size(*account_balance, signal.price, stop_loss).await;

        if position_size > Decimal::ZERO {
            let order = OrderReq {
                symbol: signal.symbol.clone(),
                id: Uuid::new_v4().to_string(),
                side: Side::Buy,
                order_type: OrderType::Market,
                size: position_size,
                price: signal.price,
                sl: Some(stop_loss),
                tp: Some(take_profit),
                manual: false
            };
            self.order_tx.send(order).await?;
        }
        Ok(())
    }

    pub async fn place_manual_order(&self, order: OrderReq) -> Result<()> {
        let mut manual_order = order;
        manual_order.manual = true;
        self.order_tx.send(manual_order).await?;
        Ok(())
    }

    pub async fn execute_order(&self, order: OrderReq) -> Result<()> {
        if matches!(order.order_type, OrderType::Limit) {
            self.binance_client.place_limit_order(&order).await?;
            println!("Placed limit order for: {}", order.id);
        }
        else if matches!(order.order_type, OrderType::Market) {
            self.binance_client.place_market_order(&order).await?;
            println!("Placed market order for: {}", order.id);
        }

        Ok(())
    }
}
