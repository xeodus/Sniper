use async_trait::async_trait;
use chrono::Utc;
use futures_util::SinkExt;
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message, 
    MaybeTlsStream, WebSocketStream
};
use crate::{data::*, exchange::{config::Exchangecfg,
    RestClient, StreamBook}, 
    utils::signature
};

pub struct KuCoin {
    pub http: Client,
    pub cfg: Exchangecfg,
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>
}

impl KuCoin {
    pub async fn new(cfg: Exchangecfg, symbol: &str) -> anyhow::Result<Self> {
        let token = KuCoin::get_ws_token(&cfg).await?;
        let http = Client::new();
        let ws_url = format!("wss://ws-api.kucoin.com/endpoint?token={}", token);
        let (ws, _) = connect_async(ws_url).await?;
        let mut kc = KuCoin { http, ws, cfg };
        let _ = kc.subscribe(symbol).await;
        Ok(kc)
    }

    pub async fn get_ws_token(cfg: &Exchangecfg) -> anyhow::Result<String> {
        let url = "https://api.kucoin.com/api/v1/bullet-private";
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(cfg.secret_key.as_bytes(), &format!("{}{}", now, "GET/api/v1/bullet-private"));
        let client = Client::new();
        let response = client.post(url)
            .header("KC-API-KEY", &cfg.api_key)
            .header("KC-API-SIGN", sign)
            .header("KC-SECRET-KEY", cfg.secret_key.clone())
            .header("KC-API-PASSPHRASE", cfg.passphrase.clone())
            .header("KC-API-VERSION", "2")
            .send()
            .await?;

        let response_json = response.json::<serde_json::Value>().await?;
        Ok(response_json["data"]["token"].to_string())
    }

    async fn subscribe(&mut self, symbol: &str) -> anyhow::Result<()> {
        let msg = json!({
            "id": 1,
            "type": "subscribe",
            "topic": format!("/market/ticker:{}", symbol),
            "response": true
        });
        let _ = self.ws.send(Message::text(msg.to_string())).await;
        Ok(())
    }
}

#[async_trait]
impl StreamBook for KuCoin {
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
                        timestamp: d["timestamp"].as_i64().unwrap()
                    });
                }
            }
        }
    }
}

#[async_trait]
impl RestClient for KuCoin {
    async fn place_order(&self, req: &OrderReq) -> anyhow::Result<()> {
        let body = json!({
            "clienOid": req.id.to_string(),
            "price": req.price.to_string(),
            "type": "limit",
            "quantity": req.quantity.to_string(),
            "side": match req.side {
                Side::Buy => "Buy",
                Side::Sell => "Sell"
            },
        });

        let url = "https://api.kucoin.com/api/v1/orders";
        let body_str = body.to_string();
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "POST", "/api/v1/orders", body_str));

        let response = self.http.post(url)
            .header(CONTENT_TYPE, "/application/json")
            .header("KC-API-KEY", &self.cfg.api_key)
            .header("KC-API-SIGN", sign)
            .header("KC-API-TIMESTAMP", now)
            .header("KC-SECRET-KEY", &self.cfg.secret_key)
            .header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("KC-API-VERSION", "2")
            .body(body_str)
            .send()
            .await?;

        response.json::<serde_json::Value>().await?;
        Ok(())
    }

    async fn cancel_order(&self, id: &str) -> anyhow::Result<()> {
        let url = format!("https://api.kucoin.com/api/v1/orders/{}", id);
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "DELETE", format!("/api/v1/orders/{}", id), ""));
        
        self.http.delete(url)
            .header("KC-API-KEY", &self.cfg.api_key)
            .header("KC-API-TIMESTAMP", now)
            .header("KC-API-SIGN", sign)
            .header("KC-SECRET-KEY", &self.cfg.secret_key)
            .header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("KC-API-VERSION", "2")
            .send()
            .await?;

        Ok(())
    }
}