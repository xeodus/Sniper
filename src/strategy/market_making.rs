use uuid::Uuid;
use crate::data::*;
use crate::strategy::trade_strategy::StrategyCalculations;
pub struct MM;

impl MM {
    pub fn new() -> Self { MM }

    pub fn decide(&mut self, prices: Vec<f64>, exchange: Exchange, tob: &TopOfBook) -> Option<OrderReq> {
        let spread = (tob.ask - tob.bid) / tob.bid;
        if spread < 0.001 { return None; }
        let mid = (tob.bid + tob.ask) / 2.0;
        let target = 0.01;
        
        let short_period = 12;
        let long_period = 26;
        let rsi_period = 14;
        let signal_period = 9;
        let boll_period = 20;
        let std_dev = 2.0;

        let short_ema = TechnicalIndicators::calculate_ema(&prices, short_period);
        let long_ema = TechnicalIndicators::calculate_ema(&prices, long_period);
        let rsi = TechnicalIndicators::calculate_rsi(&prices, rsi_period);
        let macd_map = TechnicalIndicators::calculate_macd(&prices,
            short_period, long_period, signal_period);
        let bands = TechnicalIndicators::set_bollinger_bands(&prices, 
            boll_period, std_dev);

        let short_ema_val = short_ema.last().unwrap();
        let prev_short_ema = short_ema[short_ema.len() - 2];
        let long_ema_val = long_ema.last().unwrap();
        let prev_long_ema = long_ema[long_ema.len() - 2];
        let rsi_val = rsi.last().unwrap();
        let macd_line = macd_map.get("macd_line").unwrap();
        let signal_line = macd_map.get("signal").unwrap();
        let macd_val = macd_line.last().unwrap();
        let prev_macd = macd_line[macd_line.len() - 2];
        let signal_val = signal_line.last().unwrap();
        let prev_signal = signal_line[signal_line.len() - 2];
        //let upper_band = bands.get("upper").unwrap().last().unwrap();
        let lower_band = bands.get("lower").unwrap().last().unwrap();
        let latest_price = prices[prices.len() - 1];

        if prev_short_ema <= prev_long_ema && short_ema_val > long_ema_val {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 + target/2.0),
                quantity: 0.001,
                side : Side::Buy,
                timestamp: tob.timestamp
            });
        }
        else if prev_short_ema >= prev_long_ema && short_ema_val < long_ema_val {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 - target/2.0),
                quantity: 0.001,
                side: Side::Sell,
                timestamp: tob.timestamp
            });
        }

        if *rsi_val < 30.0 {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 + target/2.0),
                quantity: 0.001,
                side: Side::Buy,
                timestamp: tob.timestamp
            });
        }
        else if *rsi_val > 70.0 {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 - target/2.0),
                quantity: 0.001,
                side: Side::Sell,
                timestamp: tob.timestamp
            });
        }

        if prev_macd <= prev_signal && macd_val > signal_val {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 + target/2.0),
                quantity: 0.001,
                side: Side::Buy,
                timestamp: tob.timestamp
            });
        }
        else if prev_macd >= prev_signal && macd_val < signal_val {
            return Some(OrderReq {
                id:Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 - target/2.0),
                quantity: 0.001,
                side: Side::Sell,
                timestamp: tob.timestamp
            });
        }
        
        if latest_price < *lower_band {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 + target/2.0),
                quantity: 0.001,
                side: Side::Buy,
                timestamp: tob.timestamp
            });
        }
        else {
            return Some(OrderReq {
                id: Uuid::new_v4().to_string(),
                exchange,
                symbol: tob.symbol.clone(),
                price: mid * (1.0 - target/2.0),
                quantity: 0.001,
                side: Side::Sell,
                timestamp: tob.timestamp
            });
        }  
    }
}
