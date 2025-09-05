use async_trait::async_trait;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::json;
use tokio::net::TcpStream;
use anyhow::Result;
use tokio_tungstenite::{connect_async, tungstenite::Message, 
    MaybeTlsStream, WebSocketStream
};
use uuid::Uuid;

use crate::{data::*, exchange::{config::Exchangecfg,
    RestClient}, utils::signature};

pub struct KuCoin {
    pub http: Client,
    pub cfg: Exchangecfg,
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>
}

impl KuCoin {

    pub fn new(cfg: Exchangecfg, ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self {
            http: Client::new(),
            cfg,
            ws
        }
    }

    pub async fn ws_connect(req: &OrderReq) -> Result<()> {
        let url = "https://api-futures.kucoin.com/api/v1/bullet-private";
        let (ws_stream, _) = connect_async(url).await?;
        let (mut tx, mut rx) = ws_stream.split();

        let subscribe = json!({
            "id": Uuid::new_v4().to_string(),
            "type": "subscriber",
            "topic": format!("/market/level2: {}", req.id.to_string()),
            "response": true
        });

        tx.send(Message::Text(subscribe.to_string())).await?;

        while let Some(msg) = rx.next().await {
            let msg_ = msg?;
            
            match msg_ {
                Message::Text(txt) => {
                    println!("{}", txt);
                },
                Message::Ping(_) | Message::Pong(_) => {},
                Message::Close(_) => {
                    log::warn!("WebSocket connection closed..");
                    break;
                },
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait]
impl RestClient for KuCoin {
    async fn place_order(&self, req: &OrderReq) -> Result<String, anyhow::Error> {
        let body = json!({
            "clientOid": req.id.to_string(),
            "symbol": req.symbol,
            "price": req.price.to_string(),
            "type": "limit",
            "leverage": 10,
            "reduceOnly": false,
            "remark": "order remarks",
            "size": req.quantity.to_string(),
            "marginMode": "ISOLATED",
            "side": match req.side {
                Side::Buy => "Buy",
                Side::Sell => "Sell"
            },
            "timestamp": req.timestamp.to_string()
        });

        let url = "https://api-futures.kucoin.com/api/v1/orders";
        let body_str = body.to_string();
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "POST", "/api/v1/orders", body_str));

        let response = self.http.post(url)
            .header(CONTENT_TYPE, "application/json")
            .header("KC-API-KEY", &self.cfg.api_key)
            .header("KC-API-SIGN", sign)
            .header("KC-API-TIMESTAMP", now)
            //.header("KC-SECRET-KEY", &self.cfg.secret_key)
            //.header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("KC-API-VERSION", "3")
            .body(body_str)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(format!(
                "Invalid response received upon placing order on KuCoin: {}", 
                response.text().await?)));
        }

        let val = response.json::<serde_json::Value>().await?;
        let res = val.to_string();
        Ok(res)
    }

    async fn cancel_order(&self, req: &OrderReq) -> Result<String, anyhow::Error> {
        let body = json!({
            "clientOid": req.id.to_string(),
            "symbol": req.symbol.to_string()
        });
        let body_str = body.to_string();
        let url = "https://api-futures.kucoin.com";
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "DELETE", format!("/api/v1/client-order/{}", req.id.to_string()), body_str));
        
        let response = self.http.delete(url)
            .header("KC-API-KEY", &self.cfg.api_key)
            .header("KC-API-TIMESTAMP", now)
            .header("KC-API-SIGN", sign)
            //.header("KC-SECRET-KEY", &self.cfg.secret_key)
            //.header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("KC-API-VERSION", "3")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(format!("Cannot cancel the order: {}", response.text().await?)));
        }

        let val = response.json::<serde_json::Value>().await?;
        let res = val.to_string();

        Ok(res)
    }
}
