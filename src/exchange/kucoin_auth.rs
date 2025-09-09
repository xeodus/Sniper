use std::collections::{HashMap, VecDeque};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::{json, Value};
use anyhow::Result;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{data::*, exchange::{config::Exchangecfg,
    RestClient}, utils::signature};

pub struct KuCoin {
    pub http: Client,
    pub cfg: Exchangecfg,
}

impl KuCoin {

    pub fn new(cfg: Exchangecfg) -> Self 
    {
        Self {
            http: Client::new(),
            cfg
        }
    }

    async fn handle_kucoin_order_update(&mut self, ordermap: &mut HashMap<String, GridOrder>, data: &Value, symbol: &str) {
        let client_oid = ordermap.get("client_oid").and_then(|v| Some(v.client_oid.clone())).unwrap();
        //let side = ordermap.get("side").and_then(|v| Some(v.side)).unwrap();
        let status = data.get("status").and_then(|v| v.as_str()).unwrap();

        match status {
            "filled" => OrderStatus::Filled,
            "new" => OrderStatus::New,
            "rejected" => OrderStatus::Rejected,
            &_ => {
                log::warn!("Unknown order status received, marking as rejected..");
                OrderStatus::Rejected
            }
        };

        if status == "filled" || status == "sent" {
            if let Some(order) = ordermap.get(&client_oid) {
                log::info!("Grid order placed: {:?}", order);

                let opposite_side = match order.side {
                    Side::Buy => "Sell",
                    Side::Sell => "Buy"
                };

                let next_level = if opposite_side == "Sell" {
                    order.level * 1.01
                }
                else {
                    order.level * 0.99
                };

                let req = OrderReq {
                    id: client_oid.clone(),
                    exchange: Exchange::KuCoin,
                    symbol: symbol.to_string(),
                    type_: "limit".into(),
                    price: next_level,
                    quantity: 0.001,
                    side: order.side.clone(),
                    timestamp: Utc::now().timestamp_millis()
                };

                if let Err(e) = self.place_order(&req).await {
                    log::warn!("Failed to place order: {}", e);
                }
                else {
                    ordermap.insert(
                        client_oid.clone(),
                        GridOrder {
                            client_oid,
                            level: next_level,
                            symbol: symbol.to_string(),
                            side: order.side.clone(),
                            active: true,
                            quantity: 0.001,
                            status: match status {
                                "filled" => OrderStatus::Filled,
                                "new" => OrderStatus::New,
                                &_ => {

                                    log::warn!("Unknown order status received, marking as rejected..");
                                    OrderStatus::Rejected
                                }
                            }
                        });
                }
            }
        }
        else if status == "rejected" {
            log::warn!("Order Rejected on KuCoin!");
        }
    }

    pub async fn ws_connect(&mut self, req: &OrderReq) -> Result<()> {
        let url = "https://api-futures.kucoin.com/api/v1/bullet-private";
        let (ws_stream, _) = connect_async(url).await?;
        let (mut tx, mut rx) = ws_stream.split();
        let topic = format!("/market/candles: {} {}", req.symbol, req.timestamp);
        let subscribe = json!({
            "id": Uuid::new_v4().to_string(),
            "type": "subscriber",
            "topic": topic,
            "response": true
        });

        tx.send(Message::Text(subscribe.to_string().into())).await?;
        log::info!("Subcribe to: {}", topic);

        const MAX_CANDLES: usize = 100;
        let mut grid_orders: HashMap<String, GridOrder> = HashMap::new();
        let mut grid_active = false;
        let mut candles: VecDeque<Candles> = VecDeque::with_capacity(MAX_CANDLES);
        let mut trend = TrendDetector::new(12, 26, 14, 0.6);

        while let Some(msg) = rx.next().await {
            let msg_ = msg?;
            
            if let Message::Text(txt) = msg_ {
                let val: Value = match serde_json::from_str(&txt) {
                    Ok(val) => val,
                    Err(_) => continue
                };
                
                if let Some(topic_v) = val.get("topic").and_then(|v| v.as_str()) {
                    if topic_v.starts_with("/market/candles") {
                        if let Some(data) = val.get("data") {
                            if let Some(arr) = data.as_array() {
                                let c = arr;
                                if c.len() <= 6 {
                                    let candle = Candles {
                                        timestamp: c[0].as_str().unwrap().parse().unwrap_or(0),
                                        open: c[1].as_str().unwrap().parse().unwrap_or(0.0),
                                        high: c[2].as_str().unwrap().parse().unwrap_or(0.0),
                                        low: c[3].as_str().unwrap().parse().unwrap_or(0.0),
                                        close: c[4].as_str().unwrap().parse().unwrap_or(0.0),
                                        volume: c[5].as_str().unwrap().parse().unwrap_or(0.0)
                                    };
                                    if candles.len() == MAX_CANDLES { candles.pop_front(); }
                                    candles.push_back(candle.clone());
                                    let (trend, _, ema_slow, atr) = trend.update(&candle);

                                    match trend {
                                        Trend::SideChop => {
                                            if !grid_active {
                                                let center = ema_slow;
                                                let half = 4.0 * atr;
                                                let grid_upper = center + half;
                                                let grid_lower = center - half;
                                                let grid_level = TrendDetector::compute_geometric_levels(grid_lower, grid_upper, 10);
                                                
                                                for level in &grid_level {
                                                    let side = if *level < center { "Buy" } else { "Sell" };
                                                    let client_oid = Uuid::new_v4().to_string();
                                                    let req = OrderReq {
                                                        id: client_oid.clone(),
                                                        exchange: Exchange::KuCoin,
                                                        symbol: req.symbol.clone(),
                                                        type_: "limit".into(),
                                                        price: req.price,
                                                        quantity: req.quantity,
                                                        side: match side {
                                                            "Buy" => Side::Buy,
                                                            "Sell" => Side::Sell,
                                                            &_ => todo!()
                                                        },
                                                        timestamp: Utc::now().timestamp_millis()
                                                    };

                                                    if let Err(e) = self.place_order(&req).await {
                                                        log::error!("Unable to place the order on KuCoin: {}", e);
                                                    }
                                                    else {
                                                        grid_orders.insert(
                                                            client_oid.clone(),
                                                            GridOrder {
                                                                client_oid: client_oid,
                                                                level: *level,
                                                                symbol: req.symbol.clone(),
                                                                side: match side {
                                                                    "Buy" => Side::Buy,
                                                                    "Sell" => Side::Sell,
                                                                    &_ => todo!()
                                                                },
                                                                active: true,
                                                                quantity: 0.001,
                                                                status: OrderStatus::New
                                                            }
                                                        );
                                                    }
                                                }
                                                grid_active = true;
                                                log::info!("Grid enabled with levels: {}", grid_level.len());
                                            }
                                        },
                                        Trend::UpTrend | Trend::DownTrend => {
                                            if grid_active {
                                                for (id, order) in grid_orders.iter() {
                                                    let _ = self.cancel_order(&req).await;
                                                    log::info!("Cancelled order at level: {} for id: {}", order.level, id);
                                                }
                                            }
                                            grid_orders.clear();
                                            grid_active = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    else if topic.contains("order") {
                        if let Some(data) = val.get("data") {
                            self.handle_kucoin_order_update(&mut grid_orders, data, &req.symbol).await;
                        }
                    }
                }
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
            "side": match req.side {
                Side::Buy => "buy",
                Side::Sell => "sell"
            },
            "type": "limit",
            "price": req.price.to_string(),
            "size": req.quantity.to_string(),
            "timeInForce": "GTC"
        });

        let url = "https://api.kucoin.com/api/v1/orders";
        let body_str = body.to_string();
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}{}", now, "POST", "/api/v1/orders", body_str));

        let response = self.http.post(url)
            .header(CONTENT_TYPE, "application/json")
            .header("KC-API-KEY", &self.cfg.api_key)
            .header("KC-API-SIGN", sign)
            .header("KC-API-TIMESTAMP", now)
            .header("KC-API-PASSPHRASE", "") // Add passphrase if needed
            .header("KC-API-VERSION", "2")
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
        let url = format!("https://api.kucoin.com/api/v1/orders/{}", req.id);
        let now = Utc::now().timestamp_millis().to_string();
        let sign = signature(self.cfg.secret_key.as_bytes(),
            &format!("{}{}{}", now, "DELETE", format!("/api/v1/orders/{}", req.id)));
        
        let response = self.http.delete(&url)
            .header("KC-API-KEY", &self.cfg.api_key)
            .header("KC-API-TIMESTAMP", now)
            .header("KC-API-SIGN", sign)
            .header("KC-API-PASSPHRASE", "") // Add passphrase if needed
            .header("KC-API-VERSION", "2")
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
