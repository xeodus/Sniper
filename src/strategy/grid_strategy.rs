use crate::data::{Candles, Trend, Side, GridOrder, OrderStatus};
use crate::strategy::trade_strategy::StrategyCalculations;
use uuid::Uuid;

/// Grid trading strategy implementation
pub struct GridStrategy {
    pub grid_levels: Vec<f64>,
    pub active_orders: Vec<GridOrder>,
    pub center_price: f64,
    pub grid_spacing: f64,
    pub max_levels: usize,
}

impl GridStrategy {
    pub fn new(center_price: f64, grid_spacing: f64, max_levels: usize) -> Self {
        Self {
            grid_levels: Vec::new(),
            active_orders: Vec::new(),
            center_price,
            grid_spacing,
            max_levels,
        }
    }

    /// Initialize grid levels based on current market conditions
    pub fn initialize_grid(&mut self, candles: &[Candles], trend: Trend) {
        if candles.is_empty() {
            return;
        }

        let current_price = candles.last().unwrap().close;
        self.center_price = current_price;

        // Calculate grid levels based on trend
        match trend {
            Trend::SideChop => {
                // Create symmetric grid around current price
                self.grid_levels = self.create_symmetric_grid(current_price);
            }
            Trend::UpTrend => {
                // Create grid with more buy levels below current price
                self.grid_levels = self.create_uptrend_grid(current_price);
            }
            Trend::DownTrend => {
                // Create grid with more sell levels above current price
                self.grid_levels = self.create_downtrend_grid(current_price);
            }
        }

        log::info!("Initialized grid with {} levels around price {}", 
                   self.grid_levels.len(), current_price);
    }

    /// Create symmetric grid for sideways markets
    fn create_symmetric_grid(&self, center_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        let half_levels = self.max_levels / 2;

        // Buy levels below center
        for i in 1..=half_levels {
            let level = center_price * (1.0 - (self.grid_spacing * i as f64));
            levels.push(level);
        }

        // Sell levels above center
        for i in 1..=half_levels {
            let level = center_price * (1.0 + (self.grid_spacing * i as f64));
            levels.push(level);
        }

        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    /// Create grid optimized for uptrend
    fn create_uptrend_grid(&self, center_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        let buy_levels = (self.max_levels as f64 * 0.7) as usize;
        let sell_levels = self.max_levels - buy_levels;

        // More buy levels below center
        for i in 1..=buy_levels {
            let level = center_price * (1.0 - (self.grid_spacing * i as f64));
            levels.push(level);
        }

        // Fewer sell levels above center
        for i in 1..=sell_levels {
            let level = center_price * (1.0 + (self.grid_spacing * i as f64));
            levels.push(level);
        }

        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    /// Create grid optimized for downtrend
    fn create_downtrend_grid(&self, center_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        let sell_levels = (self.max_levels as f64 * 0.7) as usize;
        let buy_levels = self.max_levels - sell_levels;

        // Fewer buy levels below center
        for i in 1..=buy_levels {
            let level = center_price * (1.0 - (self.grid_spacing * i as f64));
            levels.push(level);
        }

        // More sell levels above center
        for i in 1..=sell_levels {
            let level = center_price * (1.0 + (self.grid_spacing * i as f64));
            levels.push(level);
        }

        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    /// Generate grid orders for all levels
    pub fn generate_grid_orders(&self, symbol: &str, quantity: f64) -> Vec<GridOrder> {
        let mut orders = Vec::new();

        for level in &self.grid_levels {
            let side = if *level < self.center_price {
                Side::Buy
            } else {
                Side::Sell
            };

            let order = GridOrder {
                client_oid: Uuid::new_v4().to_string(),
                symbol: symbol.to_string(),
                level: *level,
                side,
                quantity,
                active: true,
                status: OrderStatus::New,
            };

            orders.push(order);
        }

        orders
    }

    /// Update grid based on filled orders
    pub fn update_grid_on_fill(&mut self, filled_order: &GridOrder) -> Option<GridOrder> {
        // Remove the filled order from active orders
        self.active_orders.retain(|order| order.client_oid != filled_order.client_oid);

        // Add opposite order at next level
        let opposite_side = match filled_order.side {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        };

        let next_level = match opposite_side {
            Side::Buy => filled_order.level * (1.0 - self.grid_spacing),
            Side::Sell => filled_order.level * (1.0 + self.grid_spacing),
        };

        let opposite_order = GridOrder {
            client_oid: Uuid::new_v4().to_string(),
            symbol: filled_order.symbol.clone(),
            level: next_level,
            side: opposite_side,
            quantity: filled_order.quantity,
            active: true,
            status: OrderStatus::New,
        };

        self.active_orders.push(opposite_order.clone());
        log::info!("Added opposite order at level {} for filled order at {}", 
                   next_level, filled_order.level);
        
        Some(opposite_order)
    }

    /// Check if grid should be adjusted based on trend change
    pub fn should_adjust_grid(&self, new_trend: Trend, current_price: f64) -> bool {
        let price_change = (current_price - self.center_price).abs() / self.center_price;
        
        // Adjust if trend changed or price moved significantly
        price_change > self.grid_spacing * 2.0
    }

    /// Get active orders that need to be placed
    pub fn get_pending_orders(&self) -> Vec<&GridOrder> {
        self.active_orders.iter()
            .filter(|order| order.status == OrderStatus::New && order.active)
            .collect()
    }

    /// Get orders that need to be cancelled
    pub fn get_orders_to_cancel(&self) -> Vec<&GridOrder> {
        self.active_orders.iter()
            .filter(|order| order.status == OrderStatus::New && !order.active)
            .collect()
    }

    /// Calculate current grid PnL
    pub fn calculate_grid_pnl(&self, current_price: f64) -> f64 {
        let mut pnl = 0.0;

        for order in &self.active_orders {
            if order.status == OrderStatus::Filled {
                match order.side {
                    Side::Buy => {
                        pnl += (current_price - order.level) * order.quantity;
                    }
                    Side::Sell => {
                        pnl += (order.level - current_price) * order.quantity;
                    }
                }
            }
        }

        pnl
    }
}