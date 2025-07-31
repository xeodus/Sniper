use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use crate::{data::*, exchange::{config::Exchangecfg, RestClient, StreamBook}, utils::signature};

pub struct Binance {
    pub http: Client,
    pub cfg: Exchangecfg,
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>
}

impl Binance {
    pub async fn new(cfg: Exchangecfg, symbol: &str) -> anyhow::Result<Self> {
        let url = format!("wss://stream.binance.com:9443/ws/{}@bookTicker",
            symbol.to_lowercase());
        let (ws, _) = connect_async(url).await?;

        Ok(Self {
            http: Client::new(),
            cfg,
            ws
        })
    }
}

#[async_trait]
impl StreamBook for Binance {
    async fn next_tob(&mut self) -> anyhow::Result<TopOfBook> {
        loop {
            if let Some(Ok(Message::Text(t))) = self.ws.next().await {
                let value: serde_json::Value = serde_json::from_str(&t)?;
                if value["type"] == "message" {
                    let d = &value["data"];
                    return Ok(TopOfBook {
                        exchange: Exchange::KuCoin,
                        symbol: d["symbol"].to_string(),
                        bid: d["bestBid"].as_f64().unwrap(),
                        ask: d["bestAsk"].as_f64().unwrap(),
                        //timestamp: d["timestamp"].as_i64().unwrap()
                    });
                }
            }
        }
    }
}

#[async_trait]
impl RestClient for Binance {
    async fn place_order(&self, req: &OrderReq) -> anyhow::Result<()> {
        let body = json!({
            "clienOid": req.id.to_string(),
            "symbol": req.symbol,
            "price": req.price.to_string(),
            "type": "limit",
            "quantity": req.quantity.to_string(),
            "side": match req.side {
                Side::Buy => "Buy",
                Side::Sell => "Sell"
            },
        });

        let url = "https://api.binance.com/api/v1/orders";
        let body_str = body.to_string();
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "POST", "/api/v1/orders", body_str));
        let response = self.http.post(url)
            .header(CONTENT_TYPE, "/application/json")
            .header("BNB-API-KEY", &self.cfg.api_key)
            .header("BNB-API-SIGN", sign)
            .header("BNB-API-TIMESTAMP", now)
            .header("BNB-SECRET-KEY", &self.cfg.secret_key)
            //.header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("BNB-API-VERSION", "2")
            .body(body_str)
            .send()
            .await?;

        response.json::<serde_json::Value>().await?;
        Ok(())
    }

    async fn cancel_order(&self, id: &str) -> anyhow::Result<()> {
        let url = format!("https://api.binance.com/api/v1/orders/{}", id);
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "DELETE", format!("/api/v1/orders/{}", id), ""));
        
        self.http.delete(url)
            .header("BNB-API-KEY", &self.cfg.api_key)
            .header("BNB-API-TIMESTAMP", now)
            .header("BNB-API-SIGN", sign)
            .header("BNB-SECRET-KEY", &self.cfg.secret_key)
            //.header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("BNB-API-VERSION", "2")
            .send()
            .await?;

        Ok(())
    }
}
