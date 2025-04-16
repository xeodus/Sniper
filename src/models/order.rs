enum Side {
    BUY,
    SELL
}

enum OrderType {
    LIMIT,
    MARKET,
    STOP_LOSS,
    TAKE_PROFIT
}

struct Order {
    symbol: String,
    price: Option<f64>,
    quantity: f64,
    side: Side,
    order_type: OrderType,
    stop_price: Option<f64>,
    time_in_force: Option<String>,
    client_order_id: Option<String>,
    timestamp: i64
}

struct OrderResponse {
    order_id: String,
    client_order_id: String,
    symbol: String,
    status: String,
    price: f64,
    original_quantity: f64,
    executed_quantity: f64,
    side: Side,
    order_type: OrderType,
    time_in_force: String,
    timestamp: i64
}