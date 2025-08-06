#[cfg(test)]
mod tests {
    use crate::{data::TechnicalIndicators,
        strategy::trade_strategy::StrategyCalculations
    };

    #[test]
    fn test_sma_empty_input() {
        let prices: Vec<f64> = vec![];
        let sma = TechnicalIndicators::calculate_sma(&prices, 3);
        assert_eq!(sma, vec![] as Vec<f64>);
    }

    #[test]
    fn test_sma_calculations() { 
        let prices = vec![5.0, 7.0, 13.0, 16.0, 18.0, 22.0, 25.0, 29.0];
        let sma = TechnicalIndicators::calculate_sma(&prices, 5);
        println!("SMA: {:?}", sma);
        assert_eq!(sma.len(), 4);
        assert_eq!(sma[0], 11.8);
        assert_eq!(sma[1], 15.2);
        assert_eq!(sma[2], 18.8);
        assert_eq!(sma[3], 22.0);
    }

    #[test]
    fn test_ema_empty_input() {
        let prices: Vec<f64> = vec![];
        let ema = TechnicalIndicators::calculate_ema(&prices, 3);
        assert_eq!(ema, vec![] as Vec<f64>);
    }

    #[test]
    fn test_ema_calculations() {
        let prices = vec![5.0, 7.0, 13.0, 16.0, 18.0, 22.0, 25.0, 29.0];
        let ema = TechnicalIndicators::calculate_ema(&prices, 5);
        println!("EMA: {:?}", ema);
        assert_eq!(ema.len(), 4);
        assert_eq!(ema[0], 11.8);
        assert_eq!(ema[1], 15.200000000000001);
        assert_eq!(ema[2], 18.46666666666667);
        assert_eq!(ema[3], 21.97777777777778)
    }

    #[test]
    fn test_rsi_empty_input() {
        let prices: Vec<f64> = vec![];
        let rsi = TechnicalIndicators::calculate_rsi(&prices, 3);
        assert_eq!(rsi, vec![] as Vec<f64>);
    }

    #[test]
    fn test_insufficient_rsi_input() {
        let prices: Vec<f64> = vec![1.0, 3.0];
        let rsi = TechnicalIndicators::calculate_rsi(&prices, 3);
        assert_eq!(rsi, vec![] as Vec<f64>);
    }

}
