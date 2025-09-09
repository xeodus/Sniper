use crate::data::{Candles, Trend, Side};
use crate::indicators::TechnicalIndicators;

/// Strategy calculations for trading decisions
pub struct StrategyCalculations;

impl StrategyCalculations {
    /// Calculate trend strength based on multiple indicators
    pub fn calculate_trend_strength(candles: &[Candles]) -> f64 {
        if candles.len() < 20 {
            return 0.0;
        }

        let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let rsi = TechnicalIndicators::calculate_rsi(&prices, 14);
        let (_, sma_20, _) = TechnicalIndicators::calculate_bollinger_bands(&prices, 20, 2.0);
        let (macd, signal, _) = TechnicalIndicators::calculate_macd(&prices, 12, 26, 9);

        if rsi.is_empty() || sma_20.is_empty() || macd.is_empty() {
            return 0.0;
        }

        let current_price = prices.last().unwrap();
        let current_sma = sma_20.last().unwrap();
        let current_rsi = rsi.last().unwrap();
        let current_macd = macd.last().unwrap();
        let current_signal = signal.last().unwrap();

        let mut strength = 0.0;

        // Price vs SMA
        if current_price > current_sma {
            strength += 0.3;
        } else {
            strength -= 0.3;
        }

        // RSI momentum
        if current_rsi > 70.0 {
            strength -= 0.2; // Overbought
        } else if current_rsi < 30.0 {
            strength += 0.2; // Oversold
        }

        // MACD signal
        if current_macd > current_signal {
            strength += 0.2;
        } else {
            strength -= 0.2;
        }

        // Volume confirmation (if available)
        if candles.len() > 1 {
            let current_volume = candles.last().unwrap().volume;
            let avg_volume: f64 = candles.iter().map(|c| c.volume).sum::<f64>() / candles.len() as f64;
            if current_volume > avg_volume * 1.2 {
                strength *= 1.1; // Volume confirmation
            }
        }

        strength.clamp(-1.0, 1.0)
    }

    /// Determine optimal position size based on risk management
    pub fn calculate_position_size(
        account_balance: f64,
        risk_percentage: f64,
        entry_price: f64,
        stop_loss_price: f64,
        max_position_size: f64,
    ) -> f64 {
        let risk_amount = account_balance * (risk_percentage / 100.0);
        let price_difference = (entry_price - stop_loss_price).abs();
        
        if price_difference == 0.0 {
            return 0.0;
        }

        let calculated_size = risk_amount / price_difference;
        calculated_size.min(max_position_size)
    }

    /// Calculate grid levels for grid trading strategy
    pub fn calculate_grid_levels(
        center_price: f64,
        grid_spacing: f64,
        num_levels: usize,
        side: Side,
    ) -> Vec<f64> {
        let mut levels = Vec::new();
        
        match side {
            Side::Buy => {
                for i in 1..=num_levels {
                    let level = center_price * (1.0 - (grid_spacing * i as f64));
                    levels.push(level);
                }
            }
            Side::Sell => {
                for i in 1..=num_levels {
                    let level = center_price * (1.0 + (grid_spacing * i as f64));
                    levels.push(level);
                }
            }
        }
        
        levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        levels
    }

    /// Calculate take profit and stop loss levels
    pub fn calculate_tp_sl_levels(
        entry_price: f64,
        side: Side,
        risk_reward_ratio: f64,
        atr: f64,
    ) -> (f64, f64) {
        match side {
            Side::Buy => {
                let stop_loss = entry_price - (atr * 2.0);
                let take_profit = entry_price + (atr * 2.0 * risk_reward_ratio);
                (take_profit, stop_loss)
            }
            Side::Sell => {
                let stop_loss = entry_price + (atr * 2.0);
                let take_profit = entry_price - (atr * 2.0 * risk_reward_ratio);
                (take_profit, stop_loss)
            }
        }
    }

    /// Validate trading signal based on multiple conditions
    pub fn validate_trading_signal(
        candles: &[Candles],
        trend: Trend,
        min_volume_ratio: f64,
    ) -> bool {
        if candles.len() < 10 {
            return false;
        }

        let current_candle = candles.last().unwrap();
        let avg_volume: f64 = candles.iter().map(|c| c.volume).sum::<f64>() / candles.len() as f64;
        
        // Volume check
        if current_candle.volume < avg_volume * min_volume_ratio {
            return false;
        }

        // Trend consistency check
        let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
        if prices.len() < 5 {
            return false;
        }

        let recent_prices = &prices[prices.len()-5..];
        let is_uptrend = recent_prices.windows(2).all(|w| w[1] >= w[0]);
        let is_downtrend = recent_prices.windows(2).all(|w| w[1] <= w[0]);

        match trend {
            Trend::UpTrend => is_uptrend,
            Trend::DownTrend => is_downtrend,
            Trend::SideChop => !is_uptrend && !is_downtrend,
        }
    }
}