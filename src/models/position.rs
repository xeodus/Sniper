struct Position {
    symbol: String,
    entry_price: f64,
    current_price: f64,
    quantity: f64,
    unrealised_pnl: f64,
    realised_pnl: f64,
    side: Side,
    stop_loss: Option<f64>,
    take_profit: Option<f64>
}