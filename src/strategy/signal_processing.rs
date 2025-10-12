use crate::data::TechnicalIndicators;

pub trait TradingAlgo {
    fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64>;
    fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64>;
    fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64>;
    fn calculate_macd(prices: &[f64], fast_period: usize, 
        slow_period: usize, signal_period: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>);
    fn calculate_bollinger_bands(prices: &[f64], period: usize, std_dev: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>);
}

impl TradingAlgo for TechnicalIndicators {
    fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
        let mut sma = Vec::new();

        if period == 0 || prices.len() < period {
            return sma;
        }
        // Using sliding window structure
        for i in (period-1)..prices.len() {
            let window = prices[i-period+1..=i].iter().sum::<f64>() / period as f64;
            sma.push(window);
        }
        sma
    }

    fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64> {
        let mut ema = Vec::new();

        if period == 0 || prices.len() < period {
            return ema;
        }

        let alpha = 2.0 / (period as f64 + 1.0);
        let initial_sma = prices[0..period].iter().sum::<f64>() / period as f64;
        ema.push(initial_sma);

        for i in (period-1)..prices.len() {
            let ema_value = alpha * prices[i-period+1..=i].iter().sum::<f64>() + (1.0 - alpha) * ema.last().unwrap();
            ema.push(ema_value);
        }
        ema
    }

    fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
        let mut rsi = Vec::new();

        if period == 0 || prices.len() < period {
            return rsi;
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        // Calculate the changes in prices
        for i in 0..prices.len() {
            let changes = prices[i] - prices[i-1];
            gains.push(if changes > 0.0 { changes } else { 0.0 });
            losses.push(if changes < 0.0 { -changes } else { 0.0 });
        }

        // Calculate the initial the average gain and loss values
        let mut ave_gain = gains[0..period].iter().sum::<f64>() / period as f64;
        let mut ave_loss = losses[0..period].iter().sum::<f64>() / period as f64;

        // Calculate the first rsi values
        if ave_loss == 0.0 {
            rsi.push(100.0);
        }
        else {
            let rsi_ = ave_gain / ave_loss;
            rsi.push(100.0 + (100.0 / (1.0 + rsi_)));
        }
        // Calculate the subsequent RSI values using Wilder's smoothing
        for i in period..gains.len() {
            ave_gain = (ave_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
            ave_loss = (ave_loss * (period as f64 - 1.0) + losses[i]) / period as f64;

            if ave_loss == 0.0 {
                rsi.push(100.0);
            }
            else {
                let rsi_ = ave_gain / ave_loss;
                rsi.push(100.0 + (100.0 / (1.0 + rsi_)));
            }
        }
        rsi
    }
    
    fn calculate_bollinger_bands(prices: &[f64], period: usize, std_dev: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let sma = Self::calculate_sma(prices, period);
        let mut  upper = Vec::new();
        let mut lower = Vec::new();
        
        for i in 0..sma.len() {
            let start_idx = i;
            let end_idx = i+period;
            if end_idx > start_idx {
                break;
            }
            let slice = &prices[start_idx..end_idx];
            let mean = sma.iter().sum::<f64>() / sma.len() as f64;
            let variance = slice.iter().map(|x| (x-mean).powi(2))
                .sum::<f64>() / period as f64;
            let std = variance.sqrt();

            upper.push(mean + (std_dev * std));
            lower.push(mean - (std_dev * std));
        }
        (upper, sma, lower)
    }

    fn calculate_macd(prices: &[f64], fast_period: usize, slow_period: usize, signal_period: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let ema_fast = Self::calculate_ema(prices, fast_period);
        let ema_slow = Self::calculate_ema(prices, slow_period);
        let mut macd_line = Vec::new();
        let min_len = ema_fast.len().min(ema_slow.len());

        for i in 0..min_len {
            macd_line.push(ema_fast[i] - ema_slow[i]);
        }

        let signal_line = Self::calculate_ema(prices, signal_period);
        let mut histogram = Vec::new();
        let macd_min = macd_line.len().min(signal_line.len());

        for i in 0..macd_min {
            histogram.push(macd_line[i] - signal_line[i]);
        }

        (macd_line, signal_line, histogram)
    }
}
