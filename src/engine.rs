use anyhow::Ok;

use crate::{data::{OrderReq, TechnicalIndicators}, 
    exchange::RestClient, strategy::trade_strategy::StrategyCalculations
};

pub struct Engine<C: RestClient> {
    pub client: C,
    pub paper: bool,
    pub last: Option<String>
}

impl<C: RestClient> Engine<C> {
    pub fn new(client: C, paper: bool) -> Self {
        Self { client, paper, last: None}
    }

    pub async fn handle(&mut self, req: &OrderReq, prices: &[f64]) -> anyhow::Result<()> {
        let short_period = 12;
        let long_period = 26;
        let rsi_period = 14;
        let signal_period = 9;
        let boll_period = 20;
        let std_dev = 2.0;

        let short_ema = TechnicalIndicators::calculate_ema(prices, short_period);
        let long_ema = TechnicalIndicators::calculate_ema(prices, long_period);
        let rsi = TechnicalIndicators::calculate_rsi(prices, rsi_period);
        let macd_map = TechnicalIndicators::calculate_macd(prices, short_period, long_period, signal_period);
        let bands = TechnicalIndicators::set_bollinger_bands(prices, boll_period, std_dev);

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
        let upper_band = bands.get("upper").unwrap().last().unwrap();
        let lower_band = bands.get("lower").unwrap().last().unwrap();
        let latest_price = prices[prices.len() - 1];

        if self.paper {
            tracing::info!("Paper order will be placed: {:?}", req);
            return Ok(());
        }

        if let Some(id) = &self.last {
            if prev_short_ema <= prev_long_ema && short_ema_val > long_ema_val {
                tracing::info!("Placing order based on ema signals: {:?}", req);
                return self.client.place_order(req).await;
            }
            else if prev_short_ema >= prev_long_ema && short_ema_val < long_ema_val {
                tracing::info!("Canceling order based on ema signals: {:?}", req);
                return self.client.cancel_order(id).await;
            }

            if *rsi_val < 30.0 {
                tracing::info!("Placing order based on rsi signals: {:?}", req);
                return self.client.place_order(req).await;
            }
            else if *rsi_val > 70.0 {
                tracing::info!("Canceling order based on rsi signals: {:?}", req);
                return self.client.cancel_order(id).await;
            }

            if prev_macd <= prev_signal && macd_val > signal_val {
                tracing::info!("Placing order based on macd signals: {:?}", req);
                return self.client.place_order(req).await;
            }
            else if prev_macd >= prev_signal && macd_val < signal_val {
                tracing::info!("Canceling order based on macd signals: {:?}", req);
                return self.client.cancel_order(id).await;
            }
            
            if latest_price < *lower_band {
                tracing::info!("Placing order based on bollinger band signals: {:?}", req);
                return self.client.place_order(req).await;
            }
            else if latest_price > *upper_band {
                tracing::info!("Canceling order based on bollinger band signals: {:?}", req);
                return self.client.cancel_order(id).await;
            }
        }
        Ok(())
    }
}