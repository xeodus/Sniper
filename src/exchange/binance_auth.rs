use async_trait::async_trait;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::json;
use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use crate::{data::*, exchange::{config::Exchangecfg, RestClient}, utils::signature};

pub struct Binance {
    pub http: Client,
    pub cfg: Exchangecfg,
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>
}

impl Binance {
    pub async fn new(cfg: Exchangecfg, ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self {
            http: Client::new(),
            cfg,
            ws
        }
    }

    pub async handle_order_update(
        &mut self, 
        ordermap: &mut HashMap<String, GridOrder>, 
        data: &Value, 
        symbol: &str)
    {
        let client_oid = data.get("ClientOid").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let status = data.get("data").and_then(|s| s.as_str()).unwrap_or("");
        let side = data.get("side").and_then(|v| v.as_str()).unwrap_or("");

        if status == "filled" || status == "done" {
            if let Some(order) = ordermap.get(&client_oid) {
                    
            }
        }
    }

    pub async fn ws_connect(req: &OrderReq) -> Result<()> {
        let url = "wss://ws-api.binance.com:443/ws-api/v3";
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
                    log::warn!("WebSocket connection closed");
                    break;
                },
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait]
impl RestClient for Binance {
    async fn place_order(&self, req: &OrderReq) -> Result<String, anyhow::Error> {
        let body = json!({
            "clientOid": req.id.to_string(),
            "symbol": req.symbol,
            "price": req.price.to_string(),
            "type": "limit",
            "quantity": req.quantity.to_string(),
            "side": match req.side {
                Side::Buy => "Buy",
                Side::Sell => "Sell"
            },
            "timestamp": req.timestamp.to_string()
        });

        let url = "wss://ws-api.binance.com:443/ws-api/v3";       
        let body_str = body.to_string();
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "POST", "/ws-api/v3", body_str));
        let response = self.http.post(url)
            .header(CONTENT_TYPE, "application/json")
            .header("BNB-API-KEY", &self.cfg.api_key)
            .header("BNB-API-SIGN", sign)
            .header("BNB-API-TIMESTAMP", now)
            //.header("BNB-SECRET-KEY", &self.cfg.secret_key)
            //.header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("BNB-API-VERSION", "2")
            .body(body_str)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(format!(
                "Invalid response received upon placing order on Binance: {}",
                response.text().await?)));
        }

        let val = response.json::<serde_json::Value>().await?;
        let res = val.to_string();
        Ok(res)
    }

    async fn cancel_order(&self, req: &OrderReq) -> Result<String> {
        let body = json!({
            "clientOid": req.id.to_string(),
            "symbol": req.symbol.to_string()
        });

        let body_str = body.to_string();
        let url = "https://api.binance.com/api/v3/order";
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "DELETE", format!("/api/v3/order/id={}", req.id.to_string()), body_str));
        
        let response = self.http.delete(url)
            .header("BNB-API-KEY", &self.cfg.api_key)
            .header("BNB-API-TIMESTAMP", now)
            .header("BNB-API-SIGN", sign)
            //.header("BNB-SECRET-KEY", &self.cfg.secret_key)
            //.header("KC-API-PASSPHRASE", &self.cfg.passphrase)
            .header("BNB-API-VERSION", "2")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(format!("Invalid response received while canceling the order on Binance: {}", 
                response.text().await?)));
        }

        let val = response.json::<serde_json::Value>().await?;
        let res = val.to_string();
        Ok(res)
    }
}
