
// STRATEGY

use std::collections::BTreeMap;
use crate::data::TechnicalIndicators;

pub trait StrategyCalculations {
    fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64>;
    fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64>;
    fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64>;
    fn calculate_macd(prices: &[f64], fast_period: usize, slow_period: usize, signal_period: usize) -> BTreeMap<String, Vec<f64>>;
    fn set_bollinger_bands(prices: &[f64], period: usize, std_multiplier: f64) -> BTreeMap<String, Vec<f64>>;
}

impl StrategyCalculations for TechnicalIndicators {
    fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
        let mut sma_values = Vec::new();

        if period == 0 || prices.len() < period {
            return sma_values;
        }
    
        for i in (period - 1).. prices.len() {
            let windows = prices[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
            sma_values.push(windows);        
        }
        sma_values
    }

    fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64> {
        let mut ema_values = Vec::new();
    
        if prices.len() < period {
            return ema_values;
        }
        
        let multiplier = 2.0 / (period + 1) as f64;
        let first_sma = prices[..period].iter().sum::<f64>() / period as f64;
        ema_values.push(first_sma);
    
        for i in period.. prices.len() {
            let prev_ema = ema_values.last().unwrap();
            let ema = (prices[i] - prev_ema) * multiplier + prev_ema;
            ema_values.push(ema);
        }
        ema_values
    }

    fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
        let mut rsi_values = Vec::new();
    
        if prices.len() < period + 1 {
            return rsi_values;
        }
    
        let changes: Vec<f64> = prices.windows(2)
            .map(|f| f[1] - f[0]).collect();
    
        let mut gains: Vec<f64> = Vec::new();
        let mut losses: Vec<f64> = Vec::new();
    
        for change in changes {
            gains.push(if change > 0.0 { change } else { 0.0 });
            losses.push(if change < 0.0 { -change } else { 0.0 });
        }
    
        for i in (period - 1).. gains.len() {
            let avg_gain = gains[i - period +1..=i].iter().sum::<f64>() / period as f64;
            let avg_loss = losses[i - period + 1..=i].iter().sum::<f64>() / period as f64;
    
            let rsi = if avg_loss == 0.0 {
                100.0
            }
            else {
                let rs = avg_gain / avg_loss;
                100.0 - (100.0 / (1.0 + rs))
            };
            rsi_values.push(rsi);
        }
        rsi_values
    }

    fn calculate_macd(prices: &[f64], 
        fast_period: usize, 
        slow_period: usize, 
        signal_period: usize) -> BTreeMap<String, Vec<f64>> 
    {
        let mut map = BTreeMap::new();
        let fast_ema = Self::calculate_ema(&prices, fast_period);
        let slow_ema = Self::calculate_ema(&prices, slow_period);
    
        if fast_ema.len() < slow_ema.len() {
            map.insert("macd_line".into(), Vec::new());
            map.insert("signal".into(), Vec::new());
            map.insert("histogram".into(), Vec::new());
            return map;
        }
    
        let start_idx = fast_period - slow_period;
    
        let macd_line: Vec<f64> = slow_ema.iter()
            .enumerate()
            .map(|(i, slow)| fast_ema[i + start_idx] - slow)
            .collect();
    
        let signal_line = Self::calculate_ema(&macd_line, signal_period);
    
        let histogram = macd_line.iter()
            .enumerate()
            .skip(macd_line.len() - signal_line.len())
            .zip(signal_line.iter())
            .map(|(macd, signal)| macd.1 - signal)
            .collect();
    
        map.insert("macd_line".into(), macd_line);
        map.insert("signal".into(), signal_line);
        map.insert("histogram".into(), histogram);
        map
    }

    fn set_bollinger_bands(prices: &[f64], period: usize, std_multiplier: f64) -> BTreeMap<String, Vec<f64>> {
        let sma = Self::calculate_sma(&prices, period);
        let upper = Vec::new();
        let lower = Vec::new();
        let mut upper_value = Vec::new();
        let mut lower_value = Vec::new();
        let mut bands = BTreeMap::new();
        
        if prices.len() < period {
            bands.insert("middle".to_string(), sma.clone());
            bands.insert("upper".to_string(), upper);
            bands.insert("lower".to_string(), lower);
            return bands;
        }
    
        for i in (period - 1).. prices.len() {
            let windows = &prices[i - period + 1..=i];
            let std_dev: Vec<f64> = windows.iter().map(|x| (x - sma[i - period + 1]).powi(2) as f64).collect();
            let std_dev_sum = std_dev.iter().sum::<f64>();
            upper_value.push(&sma[i - period + 1] + (std_dev_sum * std_multiplier));
            lower_value.push(&sma[i - period + 1] - (std_dev_sum * std_multiplier));
        }
        bands.insert("middle".into(), sma.clone());
        bands.insert("upper".into(), upper_value);
        bands.insert("lower".into(), lower_value);
        bands
    }

}
