use uuid::Uuid;
use crate::data::{Candles, GridOrder, GridStrategy, OrderStatus, Side, Trend};

impl GridStrategy {
    pub fn new(center_price: f64, grid_spacing: f64, max_levels: usize) -> Self {
        Self {
            grid_levels: Vec::new(),
            active_orders: Vec::new(),
            center_price,
            grid_spacing,
            max_levels
        }
    }

    pub fn initialize_grid(&mut self, candles: &[Candles], trend: Trend) {
        if candles.is_empty() {
            return;
        }

        let current_price = candles.last().unwrap().close;
        self.center_price = current_price;
        
        match trend {
            Trend::SideChop => {
                self.grid_levels = self.create_symmetric_grid(self.center_price);
            },
            Trend::UpTrend => {
                self.grid_levels = self.create_uptrend_grid(self.center_price);
            },
            Trend::DownTrend => {
                self.grid_levels = self.create_downtrend_grid(self.center_price);
            }
        }

        log::info!("Initialized grid with levels: {} for price: {}", self.grid_levels.len(), self.center_price);
    }

    pub fn create_symmetric_grid(&mut self, center_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        let half_level = self.max_levels / 2;

        // Buy levels below grid half
        for i in 1..=half_level {
            let level = center_price * (1.0 - (self.grid_spacing * i as f64));
            levels.push(level);
        }

        // Sell levels above grid half
        for i in 1..=half_level {
            let level = center_price * (1.0 + (self.grid_spacing * i as f64));
            levels.push(level);
        }

        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    pub fn create_uptrend_grid(&mut self, center_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        let buy_levels = (self.max_levels as f64 * 0.7) as usize;
        let sell_levels = self.max_levels - buy_levels;

        // More buy levels below grid half
        for i in 1..=buy_levels {
            let level = center_price * (1.0 - (self.grid_spacing * i as f64));
            levels.push(level);
        }

        // Fewer sell levels above grid half 
        for i in 1..=sell_levels {
            let level = center_price * (1.0 + (self.grid_spacing * i as f64));
            levels.push(level);
        }

        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    pub fn create_downtrend_grid(&mut self, center_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        let sell_levels = (self.max_levels as f64 * 0.7) as usize;
        let buy_levels = self.max_levels - sell_levels;

        // Fewer buy levels below the grid half
        for i in 1..=buy_levels {
            let level = center_price * (1.0 - (self.grid_spacing * i as f64));
            levels.push(level);
        }

        // More sell levels above the grid half
        for i in 1..=sell_levels {
            let level = center_price * (1.0 + (self.grid_spacing * i as f64));
            levels.push(level);
        }

        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    pub fn generate_grid_orders(&self, symbol: &str, size: f64) -> Vec<GridOrder> {
        let mut orders = Vec::new();

        for level in &self.grid_levels {
            let side = if *level < self.center_price {
                Side::Buy
            }
            else {
                Side::Sell
            };

            let client_oid = Uuid::new_v4().to_string();

            let order = GridOrder {
                client_oid,
                symbol: symbol.to_string(),
                level: *level,
                size,
                side: side.clone(),
                active: true,
                status: OrderStatus::New
            };

            orders.push(order);
        }
        orders
    }

    pub fn grid_update_on_filled(&mut self, filled_order: &GridOrder) -> Option<GridOrder> {
        self.active_orders.retain(|order| order.client_oid != filled_order.client_oid);

        let opposite_side = match filled_order.side {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy
        };

        let next_level = match opposite_side {
            Side::Buy => filled_order.level * (1.0 - self.grid_spacing),
            Side::Sell => filled_order.level * (1.0 - self.grid_spacing)
        };
        let client_oid = Uuid::new_v4().to_string();

        let opposite_order = GridOrder {
            client_oid,
            symbol: filled_order.symbol.clone(),
            level: next_level,
            size: filled_order.size,
            active: true,
            side: opposite_side.clone(),
            status: OrderStatus::New
        };

        self.active_orders.push(opposite_order.clone());
        log::info!("Grid orderbook updated for levels: {} and price: {}", self.grid_levels.len(), next_level);

        Some(opposite_order)
    }

    pub fn adjust_grid(&mut self, current_price: f64) -> bool {
        let price_change = (current_price - self.center_price).abs() / self.center_price;
        price_change > self.grid_spacing * 2.0
    }

    pub fn pending_orders(&self) -> Vec<&GridOrder> {
        self.active_orders.iter()
            .filter(|order| order.status == OrderStatus::New && order.active).collect()
    }

    pub fn get_orders_cancelled(&self) -> Vec<&GridOrder> {
        self.active_orders.iter()
            .filter(|order| order.status == OrderStatus::New && !order.active).collect()
    }

    pub fn grid_pnl(&self, current_price: f64) -> f64 {
        let mut pnl = 0.0;

        for order in &self.active_orders {
            if matches!(order.status, OrderStatus::New) {
                match order.side {
                    Side::Buy => {
                        pnl += (current_price - order.level) * order.size
                    },
                    Side::Sell => {
                        pnl += (order.level - current_price) * order.size
                    }
                }
            }
        }
        pnl
    }
}
