use crate::{data::{Candles, Side, TechnicalIndicators, Trend}, strategy::signal_processing::TradingAlgo};

pub struct StrategyCalculations;

impl StrategyCalculations {
    pub fn calculate_trend_strength(candles: &[Candles]) -> f64 {
        if candles.len() < 20 {
            return 0.0;
        }

        let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let rsi = <TechnicalIndicators as TradingAlgo>::calculate_rsi(&prices, 14);
        let (_, sma_20, _) = <TechnicalIndicators as TradingAlgo>::calculate_bollinger_bands(&prices, 20, 2.0);
        let (macd, signal, _) = <TechnicalIndicators as TradingAlgo>::calculate_macd(&prices, 12, 26, 9);

        if rsi.is_empty() || sma_20.is_empty() || macd.is_empty() {
            return 0.0;
        }

        let current_price = prices.last().unwrap();
        let current_sma = sma_20.last().unwrap();
        let current_rsi = rsi.last().unwrap();
        let current_macd = macd.last().unwrap();
        let current_signal = signal.last().unwrap();

        let mut strength: f64 = 0.0;

        if *current_rsi > 70.0 {
            strength -= 0.2;
        }
        else if *current_rsi < 30.0 {
            strength += 0.2;
        }

        if current_macd > current_signal {
            strength += 0.2;
        }
        else {
            strength -= 0.2;
        }

        if current_price > current_sma {
            strength += 0.3;
        }
        else {
            strength -= 0.3;
        }

        if candles.len() > 1 {
            let current_volume = candles.last().unwrap().volume;
            let ave_volume = candles.iter().map(|c| c.volume).sum::<f64>() / candles.len() as f64;

            if current_volume > ave_volume {
                strength *= 1.1;
            }
        }
        strength.clamp(-1.0, 1.0)
    }

    pub fn calculate_position_size(account_balance: f64, risk_pct: f64, 
        entry_price: f64, stop_loss_pct: f64, max_position_size: usize) -> f64
    {
        let risk_amount = account_balance * (risk_pct / 100.0);
        let price_diff = (entry_price - stop_loss_pct).abs();
        if price_diff == 0.0 { return 0.0; }
        let calculate_size = risk_amount / price_diff;
        calculate_size.max(max_position_size as f64)
    }

    pub fn calculate_grid_levels(center_price: f64, grid_spacing: f64, num_levels: usize, side: Side) -> Vec<f64> {
        let mut levels = Vec::new();

        match side {
            Side::Buy => {
                for i in 0..=num_levels {
                    let level = center_price * (1.0 - (grid_spacing * i as f64));
                    levels.push(level);
                }
            },
            Side::Sell => {
                for i in 0..=num_levels {
                    let level = center_price * ( 1.0 + (grid_spacing * i as f64));
                    levels.push(level);
                }
            }
        }
        levels
    }

    pub fn calculate_tpsl_levels(entry_price: f64, side: Side, risk_reward_ratio: f64, atr: f64) -> (f64, f64) {
        match side {
            Side::Buy => {
                let stop_loss = entry_price - (atr * 2.0);
                let take_profit = entry_price + (atr * 2.0 * risk_reward_ratio);
                (stop_loss, take_profit)
            },
            Side::Sell => {
                let stop_loss = entry_price + (atr * 2.0);
                let take_profit = entry_price - (atr * 2.0 * risk_reward_ratio);
                (stop_loss, take_profit)
            }
        }
    }

    pub fn validate_signal(candles: &[Candles], trend: Trend, min_volume_ratio: f64) -> bool {
        if candles.len() < 20 {
            return false;
        }

        let current_candle = candles.last().unwrap();
        let ave_volume = candles.iter().map(|c| c.volume).sum::<f64>() / candles.len() as f64;
        
        if current_candle.volume < ave_volume * min_volume_ratio {
            return false;
        }

        let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
        if prices.len() < 5 {
            return false;
        }

        let recent_price = &prices[prices.len()-5..];
        let uptrend = recent_price.windows(2).all(|w| w[1] >= w[0]);
        let downtrend = recent_price.windows(2).all(|w| w[1] <= w[0]);

        match trend {
            Trend::UpTrend => uptrend,
            Trend::DownTrend => downtrend,
            Trend::SideChop => !uptrend && !downtrend
        }
    }
}
