struct Candle {
    open_time: i64,
    close_time: i64,
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    volume: f64,
    quote_asset_volume: f64,
    number_of_trades: i64
}

struct OrderBookEntry {
    price: f64,
    quantity: f64
}

struct OrderBook {
    bids: BTreeMap<String, OrderBookEntry>,
    ask: BTreeMap<String, OrderBookEntry>,
    timestamp: i64
}

struct MarketData {
    symbol: String,
    price: f64,
    price_change_24h: f64,
    volume_change_24h: f64,
    high_24h: f64,
    low_24h: f64,
    orderbook: Option<OrderBook>,
    timestamp: i64
}
