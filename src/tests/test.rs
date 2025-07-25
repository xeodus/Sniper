#[cfg(test)]

#[test]

fn test_sma_calculations() {
    use crate::{data::TechnicalIndicators,
        strategy::trade_strategy::StrategyCalculations
    };

    let prices = vec![5.0, 7.0, 13.0, 16.0, 18.0, 22.0, 25.0, 29.0];
    let sma = <TechnicalIndicators as StrategyCalculations>::calculate_sma(&prices, 5);
    println!("EMA: {:?}", sma);
    assert_eq!(sma.len(), 4);
    assert_eq!(sma[0], 11.8);
    assert_eq!(sma[1], 15.2);
    assert_eq!(sma[2], 18.8);
    assert_eq!(sma[3], 22.0);
}
