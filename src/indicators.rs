use crate::data::Candles;

/// Technical indicators module for calculating various trading indicators
pub struct TechnicalIndicators;

impl TechnicalIndicators {
    /// Calculate Simple Moving Average (SMA)
    pub fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
        if prices.len() < period {
            return Vec::new();
        }

        let mut sma = Vec::new();
        for i in (period - 1)..prices.len() {
            let sum: f64 = prices[(i - period + 1)..=i].iter().sum();
            sma.push(sum / period as f64);
        }
        sma
    }

    /// Calculate Exponential Moving Average (EMA)
    pub fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64> {
        if prices.len() < period {
            return Vec::new();
        }

        let mut ema = Vec::new();
        let alpha = 2.0 / (period as f64 + 1.0);
        
        // Initialize with SMA for the first value
        let initial_sma: f64 = prices[0..period].iter().sum::<f64>() / period as f64;
        ema.push(initial_sma);

        // Calculate EMA for remaining values
        for i in period..prices.len() {
            let ema_value = alpha * prices[i] + (1.0 - alpha) * ema.last().unwrap();
            ema.push(ema_value);
        }
        ema
    }

    /// Calculate Relative Strength Index (RSI)
    pub fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
        if prices.len() < period + 1 {
            return Vec::new();
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        // Calculate price changes
        for i in 1..prices.len() {
            let change = prices[i] - prices[i - 1];
            gains.push(if change > 0.0 { change } else { 0.0 });
            losses.push(if change < 0.0 { -change } else { 0.0 });
        }

        let mut rsi = Vec::new();
        
        // Calculate initial average gain and loss
        let mut avg_gain: f64 = gains[0..period].iter().sum::<f64>() / period as f64;
        let mut avg_loss: f64 = losses[0..period].iter().sum::<f64>() / period as f64;

        // Calculate first RSI value
        if avg_loss == 0.0 {
            rsi.push(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            rsi.push(100.0 - (100.0 / (1.0 + rs)));
        }

        // Calculate subsequent RSI values using Wilder's smoothing
        for i in period..gains.len() {
            avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;

            if avg_loss == 0.0 {
                rsi.push(100.0);
            } else {
                let rs = avg_gain / avg_loss;
                rsi.push(100.0 - (100.0 / (1.0 + rs)));
            }
        }

        rsi
    }

    /// Calculate Bollinger Bands
    pub fn calculate_bollinger_bands(prices: &[f64], period: usize, std_dev: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let sma = Self::calculate_sma(prices, period);
        let mut upper_band = Vec::new();
        let mut lower_band = Vec::new();

        for i in 0..sma.len() {
            let start_idx = i;
            let end_idx = i + period;
            if end_idx > prices.len() {
                break;
            }

            let slice = &prices[start_idx..end_idx];
            let mean = sma[i];
            let variance: f64 = slice.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / period as f64;
            let std = variance.sqrt();

            upper_band.push(mean + (std_dev * std));
            lower_band.push(mean - (std_dev * std));
        }

        (upper_band, sma, lower_band)
    }

    /// Calculate MACD (Moving Average Convergence Divergence)
    pub fn calculate_macd(prices: &[f64], fast_period: usize, slow_period: usize, signal_period: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let ema_fast = Self::calculate_ema(prices, fast_period);
        let ema_slow = Self::calculate_ema(prices, slow_period);

        let mut macd_line = Vec::new();
        let min_len = ema_fast.len().min(ema_slow.len());

        for i in 0..min_len {
            macd_line.push(ema_fast[i] - ema_slow[i]);
        }

        let signal_line = Self::calculate_ema(&macd_line, signal_period);
        
        let mut histogram = Vec::new();
        let min_len = macd_line.len().min(signal_line.len());
        for i in 0..min_len {
            histogram.push(macd_line[i] - signal_line[i]);
        }

        (macd_line, signal_line, histogram)
    }

    /// Calculate Average True Range (ATR)
    pub fn calculate_atr(candles: &[Candles], period: usize) -> Vec<f64> {
        if candles.len() < 2 {
            return Vec::new();
        }

        let mut true_ranges = Vec::new();
        
        for i in 1..candles.len() {
            let high_low = candles[i].high - candles[i].low;
            let high_close = (candles[i].high - candles[i-1].close).abs();
            let low_close = (candles[i].low - candles[i-1].close).abs();
            
            let true_range = high_low.max(high_close).max(low_close);
            true_ranges.push(true_range);
        }

        Self::calculate_ema(&true_ranges, period)
    }
}