use crate::data::{OrderReq, Side};
use chrono::Utc;
use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::json;
use tracing::info;
use anyhow::{anyhow, Result};
use crate::sign::signature;

pub struct BinanceClient {
    pub client: Client,
    pub base_url: String,
    pub api_key: String,
    pub api_secret: String
}

impl BinanceClient {
    pub fn new(api_key: String, api_secret: String, testnet: bool) -> Self {
        let base_url = if testnet {
            "https://testnet.binance.vision".to_string()
        }
        else {
            "https://api.binance.com".to_string()
        };

        Self {
            client: Client::new(),
            base_url,
            api_key,
            api_secret
        }
    }

    pub async fn account_balance(&self) -> Result<Decimal> {
        let url = format!("{}/api/v3/account", self.base_url);
        let mock_data = signature(self.api_secret.as_bytes(), &url).await;
        info!("Fetching account details: {:?}", mock_data);
        Ok(Decimal::new(10000, 0))
    }

    pub async fn place_market_order(&self, req: &OrderReq) -> Result<String> {
        info!("Placing market order {:?} for {} of size {} @ {}", req.side, req.symbol, req.size, req.price);

        let body = json!({
            "symbol": req.symbol.to_string(),
            "side": match req.side {
                Side::Buy => "Buy".to_string(),
                Side::Sell => "Sell".to_string(),
                Side::Hold => "Hold".to_string()
            }, 
            "timeInForce": "GTC",
            "size": req.size.to_string(),
            "price": req.price.to_string(), 
            "newClientOrderId": req.id.to_string(),
            "timestamp": Utc::now().timestamp_millis().to_string() 
        });

        let url = "https://www.binance.com/api/v3/order";
        let body_str = body.to_string();
        let sign = signature(self.api_secret.as_bytes(), &body_str).await;
        let response = self.client.post(format!("{}?{}&signature={:?}", url, body_str, sign))
            .header("X-MBX-APIKEY", self.api_key.clone()).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Invalid response received while placing the order on Binance: {:?}", response.text().await));
        }

        let res = response.json::<serde_json::Value>().await?;
        Ok(res.to_string())
    }

    pub async fn place_limit_order(&self, req: &OrderReq) -> Result<String> {
        info!("placing limit order {:?} for {} of size {} @ {}", req.side, req.symbol, req.size, req.price);

        let body = json!({
            "symbol": req.symbol.clone(),
            "side": match req.side {
                Side::Buy => "Buy".to_string(),
                Side::Sell => "Sell".to_string(),
                Side::Hold => "Hold".to_string()
            }, 
            "timeInForce": "GTC",
            "size": req.size.to_string(),
            "price": req.price.to_string(),
            "newClientOrderId": req.id.to_string(),
            "timestamp": Utc::now().timestamp_millis().to_string()
        });

        let url = "https://www.binance.com/api/v3/order";
        let body_str = body.to_string();
        let sign = signature(self.api_secret.as_bytes(), &body_str).await;
        let response = self.client.post(format!("{}?{}&signature={:?}", url, body_str, sign))
            .header("X-MBX-APIKEY", self.api_key.clone()).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Invalid response received while placing the limit order on Binance: {:?}", response.text().await));
        }

        let res = response.json::<serde_json::Value>().await?;
        Ok(res.to_string())
    }

    pub async fn cancel_orders(&self, req: &OrderReq) -> Result<String> {
        info!("Cancelling the order for ID {} and symbol {}", req.id, req.symbol);
        let url = "https://api.binance.com/api/v3/order";
        let now = Utc::now().timestamp_millis().to_string();
        let query_string = format!("symbol={}&originClientOrderId={}&timestamp={}", req.symbol, req.id, now);
        let sign = signature(self.api_secret.as_bytes(), &query_string).await;
        let response = self.client.delete(format!("{}?{}&signature={}", url, query_string, sign)).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Invalid response received while cancelling the orders at Binance: {:?}", response.text().await));
        }

        let res = response.json::<serde_json::Value>().await?;
        Ok(res.to_string())
    }
}
