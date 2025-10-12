use crate::data::{Candles, Trend, TrendDetector};

impl TrendDetector {
    pub fn new(fast_period: usize, slow_period: usize, atr_period: usize, k_atr: f64) -> Self {
        let alpha_fast = 2.0 / (fast_period as f64 + 1.0);
        let alpha_slow = 2.0 / (slow_period as f64 + 1.0);
        let alpha_atr = 2.0 / (atr_period as f64 + 1.0);

        Self {
            alpha_fast,
            alpha_slow,
            alpha_atr,
            ema_slow: 0.0,
            ema_fast: 0.0,
            prev_closed: 0.0,
            atr: alpha_atr,
            initialized: false,
            k_atr
        }
    }

    pub fn update(&mut self, c: &Candles) -> (Trend, f64, f64, f64) {
        if !self.initialized {
            self.ema_fast = c.close;
            self.ema_slow = c.close;
            self.atr = (c.high - c.low).abs();
            self.prev_closed = c.close;
        }
        else {
            self.ema_slow += self.alpha_slow * (c.close - self.ema_slow);
            self.ema_fast += self.alpha_fast * (c.close - self.ema_fast);
            let atr = (c.high - c.low)
                .max((c.high - self.prev_closed).abs())
                .max((c.low - self.prev_closed).abs());
            self.atr += self.alpha_atr * (atr - self.atr).abs();
            self.prev_closed = c.close;
        }

        let diff = self.ema_fast - self.ema_slow;
        let threshold = self.k_atr * self.atr.max(10_f64.powi(-9));

        let trend = if diff > threshold {
            Trend::UpTrend
        }
        else if diff < -threshold {
            Trend::DownTrend
        }
        else {
            Trend::SideChop
        };

        (trend, self.ema_fast, self.ema_slow, self.atr)
    }

    pub fn compute_generic_levels(higher: f64, lower: f64, levels: usize) -> Vec<f64> {
        let mut output = Vec::with_capacity(levels);

        if levels == 0 || lower <= 0.0 || higher <= lower {
            return output;
        }

        for i in 0..levels {
            let frac = i as f64 / (levels as f64 - 1.0);
            let price = lower * (higher/lower).powf(frac);
            output.push(price);
        }
        output
    }
}

