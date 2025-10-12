use std::{sync::Arc};
use std::time::Duration;
use tokio::sync::{mpsc, watch, RwLock};
use chrono::Utc;
use anyhow::Result;

use crate::{config::{AppConfig, ExchangeCfg, WebSocketCfg}, 
    data::{BotState, Exchange, GridOrder, GridStrategy, OrderReq, OrderStatus, Trend}, 
    exchange::{binance_auth::BinanceAuth, config::RestClient, kucoin_auth::KuCoinAuth}, 
    store::OrderStore, websocket::{binance_ws::BinanceClient, kucoin_ws::KuCoinClient, ws_client::WebSocketClient}};

pub struct TradingEngine {
    config: AppConfig,
    exchange_type: Exchange,
    exchange_config: ExchangeCfg,
    trend: Trend,
    grid_strategy: Arc<RwLock<GridStrategy>>,
    order_store: Arc<RwLock<OrderStore>>,
    state: Arc<RwLock<watch::Sender<BotState>>>,
    order_tx: mpsc::Sender<OrderReq>,
    order_rx: Arc<RwLock<mpsc::Receiver<OrderReq>>>,
    update_tx: mpsc::Sender<GridOrder>,
    update_rx: Arc<RwLock<mpsc::Receiver<GridOrder>>>,
    ws_client: Arc<RwLock<Box<dyn WebSocketClient + Send + Sync>>>,
}

impl TradingEngine {
    pub async fn new(config: AppConfig, exchange: Exchange, trend: Trend) -> Result<Self> {
        let exchange_ = match exchange {
            Exchange::Binance => "binance",
            Exchange::KuCoin => "kucoin"
        };
        let exchange_config = config.get_exchange_config(exchange_)?;
        
        // Initialize database
        let store = OrderStore::init_db(&config.database.path)?;
        let existing_orders = OrderStore::db_load_orders(&store.conn)?;
        
        // Create shared state
        let (state_tx, _state_rx) = watch::channel(BotState {
            trend: trend.clone(),
            open_orders: existing_orders,
            pnl: 0.0,
        });

        // Create communication channels
        let (order_tx, order_rx) = mpsc::channel::<OrderReq>(100);
        let (update_tx, update_rx) = mpsc::channel::<GridOrder>(100);
        
        // Initialize WebSocket client
        let ws_config = WebSocketCfg {
            retry_interval: 5,
            max_retry_attempts: 10,
            max_candles: 20,
            heartbeat_interval: 30,
            buffer_size: 1000
        };

        let ws_client: Box<dyn WebSocketClient + Send + Sync> = match exchange {
            Exchange::Binance => Box::new(BinanceClient::new(ws_config)),
            Exchange::KuCoin => Box::new(KuCoinClient::new(ws_config))
        };

        // Initialize grid strategy
        let grid_strategy = GridStrategy::new(
            0.0,
            config.trading.grid_spacing,
            config.trading.grid_levels,
        );

        Ok(Self {
            config,
            exchange_type: exchange,
            exchange_config,
            trend,
            grid_strategy: Arc::new(RwLock::new(grid_strategy)),
            order_store: Arc::new(RwLock::new(store)),
            state: Arc::new(RwLock::new(state_tx)),
            order_tx,
            order_rx: Arc::new(RwLock::new(order_rx)),
            update_tx,
            update_rx: Arc::new(RwLock::new(update_rx)),
            ws_client: Arc::new(RwLock::new(ws_client)),
        })
    }

    pub async fn start(&self) -> Result<()> {
        log::info!("Starting trading engine for {:?}", self.exchange_type);

        // Start WebSocket connection
        self.start_websocket().await?;

        // Start order processor
        self.start_order_processor().await;

        // Start order update processor
        self.start_order_update_processor().await;

        // Start main trading loop
        self.start_trading_loop().await?;

        //Get current state
        self.get_state().await;

        // Update trend
        self.update_trend(self.trend.clone()).await?;

        // Update PnL value
        self.get_pnl().await;

        Ok(())
    }

    async fn start_websocket(&self) -> Result<()> {
        let ws_client = self.ws_client.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut client = ws_client.write().await;
            
            if let Err(e) = client.connect().await {
                log::error!("Failed to connect WebSocket: {}", e);
                return;
            }

            if let Err(e) = client.subscribe_to_candles(&config.trading.symbol, &config.trading.timeframe).await {
                log::error!("Failed to subscribe to candles: {}", e);
                return;
            }

            if let Err(e) = client.subscribe_to_orders().await {
                log::error!("Failed to subscribe to orders: {}", e);
                return;
            }

            log::info!("WebSocket connected and subscribed successfully");

            // Handle WebSocket messages
            loop {
                // This would need to be implemented based on the specific WebSocket client
                // For now, we'll simulate with a sleep
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        Ok(())
    }

    async fn start_order_processor(&self) {
        let order_rx = self.order_rx.clone();
        //let order_store = self.order_store.clone();
        let state = self.state.clone();
        let exchange_type = self.exchange_type.clone();
        let exchange_config = self.exchange_config.clone();

        tokio::spawn(async move {
            let mut rx = order_rx.write().await;
            while let Some(order_req) = rx.recv().await {
                log::info!("Processing order: {:?}", order_req);
                
                // Place order on exchange
                let result = match exchange_type {
                    Exchange::Binance => {
                        let client = BinanceAuth::new(exchange_config.clone());
                        client.place_order(&order_req).await
                    }
                    Exchange::KuCoin => {
                        let client = KuCoinAuth::new(exchange_config.clone());
                        client.place_order(&order_req).await
                    }
                };

                match result {
                    Ok(response) => {
                        log::info!("Order placed successfully: {}", response);
                        
                        // Save to database
                        let grid_order = GridOrder {
                            client_oid: order_req.id.clone(),
                            symbol: order_req.symbol.clone(),
                            level: order_req.price,
                            side: order_req.side.clone(),
                            size: order_req.size.clone(),
                            active: true,
                            status: OrderStatus::New,
                        };

                        /*let mut store = order_store.write().await;

                        if let Err(e) = store.db_save_orders(&grid_order) {
                            log::error!("Failed to save order to database: {}", e);
                        }*/

                        // Update state
                        let state_guard = state.write().await;
                        let mut current_state = state_guard.borrow().clone();
                        current_state.open_orders.push(grid_order);
                        let _ = state_guard.send(current_state);
                    },
                    Err(e) => {
                        log::error!("Failed to place order: {}", e);
                    }
                }
            }
        });
    }

    async fn start_order_update_processor(&self) {
        let update_rx = self.update_rx.clone();
        //let update_store = self.order_store.clone(); 
        let state = self.state.clone();
        let grid_strategy = self.grid_strategy.clone();
        let order_tx = self.order_tx.clone();
        let exchange_type = self.exchange_type.clone();

        tokio::spawn(async move {
            let mut rx = update_rx.write().await;
            while let Some(updated_order) = rx.recv().await {
                log::info!("Processing order update: {:?}", updated_order);
                
                // Update database
                /*let mut store = update_store.write().await;
                if let Err(e) = store.db_update_status(&updated_order).await {
                    log::error!("Failed to update order in database: {}", e);
                }*/

                // Update state_guard
                let state_guard = state.write().await;
                let mut current_state = state_guard.borrow().clone();
                if let Some(pos) = current_state.open_orders.iter().position(|o| o.client_oid == updated_order.client_oid) {
                    current_state.open_orders[pos] = updated_order.clone();
                }
                let _ = state_guard.send(current_state);

                // Handle grid strategy updates
                if updated_order.status == OrderStatus::Filled {
                    let mut strategy = grid_strategy.write().await;
                    if let Some(new_order) = strategy.grid_update_on_filled(&updated_order) {
                        // Place the new opposite order
                        let order_req = OrderReq {
                            id: new_order.client_oid.clone(),
                            exchange: exchange_type.clone(),
                            symbol: new_order.symbol.clone(),
                            type_: "limit".to_string(),
                            price: new_order.level,
                            size: new_order.size.clone(),
                            side: new_order.side.clone(),
                            timestamp: Utc::now().timestamp_millis(),
                        };

                        if let Err(e) = order_tx.send(order_req).await {
                            log::error!("Failed to send new order request: {}", e);
                        }
                    }
                }
            }
        });
    }

    async fn start_trading_loop(&self) -> Result<()> {
        let mut last_trend = Trend::SideChop;
        let grid_strategy = self.grid_strategy.clone();
        let state = self.state.clone();
        let order_tx = self.order_tx.clone();
        let exchange_type = self.exchange_type.clone();
        let config = self.config.clone();

        loop {
            // Get current state
            let current_state = {
                let state_guard = state.read().await;
                let x = state_guard.borrow().clone();
                x
            };

            // Check if trend has changed
            if current_state.trend != last_trend {
                log::info!("Trend changed from {:?} to {:?}", last_trend, current_state.trend);
                
                // Cancel existing orders if trend changed significantly
                if matches!(current_state.trend, Trend::UpTrend | Trend::DownTrend) && 
                   matches!(last_trend, Trend::SideChop) {
                    for order in &current_state.open_orders {
                        if order.status == OrderStatus::New {
                            let cancel_req = OrderReq {
                                id: order.client_oid.clone(),
                                exchange: exchange_type.clone(),
                                symbol: order.symbol.clone(),
                                type_: "limit".to_string(),
                                price: order.level,
                                size: order.size.clone(),
                                side: order.side.clone(),
                                timestamp: Utc::now().timestamp_millis(),
                            };

                            let _ = order_tx.send(cancel_req).await;
                        }
                    }
                }

                last_trend = current_state.trend.clone();
            }

            // Generate new grid orders if needed
            if matches!(current_state.trend, Trend::SideChop) && current_state.open_orders.is_empty() {
                let client_read = self.ws_client.read().await;
                let candles = client_read.get_candles().await;
                if !candles.is_empty() {
                    let mut strategy = grid_strategy.write().await;
                    strategy.initialize_grid(candles, current_state.trend.clone());
                    let new_orders = strategy.generate_grid_orders(&config.trading.symbol, config.trading.quantity);
                    
                    for order in new_orders {
                        let order_req = OrderReq {
                            id: order.client_oid.clone(),
                            exchange: exchange_type.clone(),
                            symbol: order.symbol.clone(),
                            type_: "limit".to_string(),
                            price: order.level,
                            size: order.size.clone(),
                            side: order.side.clone(),
                            timestamp: Utc::now().timestamp_millis(),
                        };

                        if let Err(e) = order_tx.send(order_req).await {
                            log::error!("Failed to send order request: {}", e);
                        }
                    }
                }
            }

            // Sleep before next iteration
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn get_state(&self) -> BotState {
        let state_guard = self.state.read().await;
        let botstate = state_guard.borrow().clone();
        botstate
    }

    pub async fn update_trend(&self, trend: Trend) -> Result<()> {
        let state_guard = self.state.write().await;
        let mut current_state = state_guard.borrow().clone();
        current_state.trend = trend;
        state_guard.send(current_state)?;
        Ok(())
    }

    pub async fn get_pnl(&self) -> f64 {
        let state = self.get_state().await;
        state.pnl
    }
}
