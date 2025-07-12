#[cfg(test)]
use crate::{CandleSticks, 
    MACStrategy, PositionSizer, 
    TechnicalIndicators, TradingStrategy
};

#[test]
fn test_sma_calculations() {
    let prices = vec![5.0, 7.0, 13.0, 16.0, 18.0];
    let sma = TechnicalIndicators::calculate_sma(&prices, 5);
    assert_eq!(sma, vec![11.8]);
}

#[test]
fn test_position_sizing() {
    let size_init = PositionSizer::init(10000.0, 0.02);
    let calculate_size = size_init.calculate_position_size(1450.0, 72.5);
    assert_eq!(calculate_size, 0.14);
}

#[test]
fn test_strategy_signals() {
    let mut strategy = MACStrategy::new(2, 5, 3);
    let candles = vec![
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 1,
            open: 1400.0,
            high: 1678.0,
            low: 1570.0,
            close: 1590.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 2,
            open: 1590.0,
            high: 1610.0,
            low: 1540.0,
            close: 1565.0,
            volume:100000.0,
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 3,
            open: 1565.0,
            high: 1620.0,
            low: 1578.0,
            close: 1590.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 4,
            open: 1590.0,
            high: 1699.0,
            low: 1679.0,
            close: 1683.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 5,
            open: 1683.0,
            high: 1750.0,
            low: 1711.0,
            close: 1711.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 6,
            open: 1711.0,
            high: 1780.0,
            low: 1758.0,
            close: 1768.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 7,
            open: 1768.0,
            high: 1810.0,
            low: 1771.0,
            close: 1783.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 8,
            open: 1783.0,
            high: 1868.0,
            low: 1833.0,
            close: 1856.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 9,
            open: 1856.0,
            high: 1923.0,
            low: 1889.0,
            close: 1889.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 10,
            open: 1889.0,
            high: 1932.0,
            low: 1910.0,
            close: 1923.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 11,
            open: 1923.0,
            high: 1957.0,
            low: 1944.0,
            close: 1946.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 12,
            open: 1946.0,
            high: 1977.0,
            low: 1965.0,
            close: 1971.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 13,
            open: 1971.0,
            high: 1997.0,
            low: 1987.0,
            close: 1989.0,
            volume: 100000.0
        },
        CandleSticks {
            symbol: "TEST".into(),
            timestamp: 14,
            open: 1989.0,
            high: 2114.0,
            low: 1998.0,
            close: 2004.0,
            volume: 100000.0
        },
    ];

    strategy.analyze_market(&candles);
}

