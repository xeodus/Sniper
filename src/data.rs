use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum Side {
    Buy,
    Sell
}

#[derive(Debug, Deserialize, Clone)]
pub enum Exchange {
    KuCoin,
    Binance
}

/*pub enum OrderStatus {
    New,
    Filled,
    Canceled,
    Rejected
}*/

#[derive(Debug, Deserialize, Clone)]
pub struct OrderReq {
    pub id: String,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub timestamp: i64
}

#[derive(Debug, Deserialize, Clone)]
pub struct TopOfBook {
    pub exchange: Exchange,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64
}

/*pub struct OrderUpdate {
    pub id: String,
    pub status: OrderStatus
}*/

pub struct TechnicalIndicators;

/*#[derive(Deserialize,Clone)]
pub struct BotState {
    pub top_of_book: Vec<TopOfBook>,
    pub orders: Vec<OrderReq>,
    pub log: Vec<String>
}

impl BotState {
    pub fn new() -> Self {
        Self {
            top_of_book: Vec::new(),
            orders: Vec::new(),
            log: Vec::new()
        }
    }

    pub fn add_tob(&mut self, tob: TopOfBook) {
        self.top_of_book.push(tob);

        if self.top_of_book.len() > 50 {
            self.top_of_book.drain(0..self.top_of_book.len() - 50);
        }
    }

    pub fn add_order(&mut self, order: OrderReq) {
        self.orders.push(order);

        if self.orders.len() > 50 {
            self.orders.drain(0..self.orders.len() - 50);
        }
    }

    pub fn add_log(&mut self, log: String) {
        self.log.push(log);

        if self.log.len() > 50 {
            self.log.drain(0..self.log.len() - 50);
        }
    }
}

pub type SharedState = Arc<Mutex<BotState>>;*/
