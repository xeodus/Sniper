use std::{env, time::{SystemTime, UNIX_EPOCH}};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use sha2::Sha256;

use crate::data::Config;

pub trait KucoinFuturesAPI {
    fn new(sandbox: bool) -> Result<Self, anyhow::Error> where Self: Sized;
    async fn signature_generation(&self, timestamp: &str, method: &str, path: &str, body: &str) -> String;
    async fn generate_passphrase(&self) -> String;
    async fn header_assembly(&self, method: &str, path: &str, body: &str) -> HeaderMap;
}

impl KucoinFuturesAPI for Config {
    fn new(sandbox: bool) -> Result<Self, anyhow::Error> 
    {
        let base_url = if sandbox {
            "https://api-sandbox-futures.kucoin.com".into()
        }
        else {
            "https://api-futures.kucoin.com".into()
        };

        Ok(
            Config {
                api_key: env::var("API_KEY").expect("API key not found.."),
                api_secret: env::var("API_SECRET").expect("API secret not found"),
                api_passphrase: env::var("API_PASSPHRASE").expect("API passphrase not found.."),
                base_url,
                sandbox,
                risk_per_trade: 0.0,
                max_drawdown: 0.0,
                account_balance: 100000.0,
                max_portfolio_risk: 0.0,
            }
        )
    }

    async fn signature_generation(&self, 
        timestamp: &str, 
        method: &str, 
        path: &str, 
        body: &str
    ) -> String 
    {
        let query_string = format!("{}{}{}{}", timestamp, method, path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(
            self.api_secret.as_bytes()
        )
        .expect("Hmac can take key of all size..");
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    async fn generate_passphrase(&self) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(
            self.api_secret.as_bytes()
        )
        .expect("Hmac can take key of all size..");
        mac.update(self.api_passphrase.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    async fn header_assembly(&self,
        method: &str,
        path: &str, 
        body: &str
    ) -> HeaderMap 
    {
        let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards..")
        .as_secs()
        .to_string();

        let mut headers = HeaderMap::new();
        let signature = self.signature_generation(
            &timestamp,
            method,
            path,
            body)
            .await;
        let passphrase = self.generate_passphrase().await;
        headers.insert("API-KEY", HeaderValue::from_str(&self.api_key).unwrap());
        headers.insert("API-SECRET", HeaderValue::from_str(&signature).unwrap());
        headers.insert("API-PASSPHRASE", HeaderValue::from_str(&passphrase).unwrap());
        headers.insert("API-KEY-TIMESTAMP", HeaderValue::from_str(&timestamp).unwrap());
        headers.insert("API-KEY-VERSION", HeaderValue::from_static("2"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}