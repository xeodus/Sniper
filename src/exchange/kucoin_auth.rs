use anyhow::Ok;
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use std::collections::{HashMap, VecDeque};
use crate::{config::ExchangeCfg, data::{Candles, Exchange, GridOrder, 
    OrderReq, OrderStatus, Side, Trend, TrendDetector}, 
    exchange::config::{signature, RestClient}
};

pub struct KuCoinAuth {
    pub http: Client,
    pub cfg: ExchangeCfg
}

impl KuCoinAuth {
    pub fn new(cfg: ExchangeCfg) -> Self {
        Self {
            http: Client::new(),
            cfg
        }
    }

    pub async fn handle_market_data_kc(&mut self, ordermap: &mut HashMap<String, GridOrder>, data: &Value, symbol: &str) {
        let client_oid = ordermap.get("client_oid").and_then(|v| Some(v.client_oid.clone())).unwrap();
        let side = ordermap.get("side").and_then(|v| Some(v.side.clone())).unwrap();
        let status = data.get("status").and_then(|v| v.as_str()).unwrap();

        match status {
            "new" => OrderStatus::New,
            "filled" => OrderStatus::Filled,
            "rejected" => OrderStatus::Rejected,
            &_ => todo!()
        };

        if status == "new" || status == "filled" {
            if let Some(order) = ordermap.get(&client_oid) {
                log::info!("Placing grid order on KuCoin: {:?}", order);
                // Placing opposite side orders
                let opposite_side = match side {
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
                    symbol: symbol.to_string(),
                    exchange: Exchange::KuCoin,
                    price: next_level,
                    size: 0.01,
                    type_: "limit".to_string(),
                    side: side.clone(),
                    timestamp: Utc::now().timestamp_millis()
                };

                if let Err(e) = self.place_order(&req).await {
                    log::warn!("Unable to place order on KuCoin exchange: {}", e);
                }
                else {
                    ordermap.insert(client_oid.clone(), 
                        GridOrder {
                            client_oid: client_oid.clone(),
                            symbol: req.symbol.clone(),
                            level: req.price,
                            size: req.size,
                            side: side.clone(),
                            active: true,
                            status: match status {
                                "new" => OrderStatus::New,
                                "filled" => OrderStatus::Filled,
                                "rejected" => OrderStatus::Rejected,
                                &_ => {
                                    log::warn!("Invalid status received from KuCoin exchange marking as rejected!");
                                    OrderStatus::Rejected
                                }
                            }
                        }
                    );
                }
            }
        }
        else if status == "rejected" {
            log::warn!("Order rejected: {:?}", OrderStatus::Rejected);
        }
    }

    pub async fn ws_connect(&mut self, req: &OrderReq) -> anyhow::Result<()> {
        let url = "https://api-futures.kucoin.com/api/v1/bullet-private";
        let (ws_stream, _) = connect_async(url).await?;
        // Channel created to send and receive ws messages
        let (mut tx, mut rx) = ws_stream.split();

        let topic = format!("/market/candles: {}{}", req.symbol, req.timestamp);
        let subscribe = json!({
            "id": Uuid::new_v4().to_string(),
            "type": "subscriber",
            "topic": topic,
            "response": true
        });

        tx.send(Message::Text(subscribe.to_string())).await?;

        log::info!("subscribed to: {}", topic);

        const MAX_CANDLES: usize = 500;
        let mut candles: VecDeque<Candles> = VecDeque::with_capacity(MAX_CANDLES);
        let mut trend = TrendDetector::new(12, 26, 14, 0.6);
        let mut grid_orders: HashMap<String, GridOrder> = HashMap::new();
        let mut grid_active = false;

        // Receiving message stream
        while let Some(msg) = rx.next().await {
            let msg_ = msg?;
            if let Message::Text(txt) = msg_ {
                // Deserialize the received json message
                let val: Value = serde_json::from_str(&txt).unwrap();

                if let Some(topic_) = val.get("topic").and_then(|v| v.as_str()) {
                    if topic_.starts_with("/markets/candles") {
                        if let Some(data) = val.get("data") {
                            if let Some(arr) = data.as_array() {
                                let c = arr; 
                                if c.len() >= 6 {
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
                                                let grid_upper = half + center;
                                                let grid_lower = center - half;
                                                let grid_level = TrendDetector::compute_generic_levels(grid_upper, 
                                                    grid_lower, 10);

                                                for level in &grid_level {
                                                    let side = if *level < center {
                                                        "Buy"
                                                    }
                                                    else {
                                                        "Sell"
                                                    };
                                                    let client_oid = Uuid::new_v4().to_string();

                                                    let req = OrderReq {
                                                        id: client_oid.clone(),
                                                        symbol: req.symbol.clone(),
                                                        exchange: Exchange::Binance,
                                                        price: req.price,
                                                        size: req.size,
                                                        type_: req.type_.clone(),
                                                        side: match side {
                                                            "Buy" => Side::Buy,
                                                            "Sell" => Side::Sell,
                                                            &_ => todo!()
                                                        },
                                                        timestamp: Utc::now().timestamp_millis()
                                                    };

                                                    if let Err(e) = self.place_order(&req).await {
                                                        log::warn!("Cannot place order on KuCoin: {}", e);
                                                    }
                                                    else {
                                                        grid_orders.insert(client_oid.clone(),
                                                            GridOrder {
                                                                client_oid,
                                                                symbol: req.symbol.clone(),
                                                                level: *level,
                                                                size: req.size,
                                                                active: true,
                                                                side: match side {
                                                                    "Buy" => Side::Buy,
                                                                    "Sell" => Side::Sell,
                                                                    &_ => todo!()
                                                                },
                                                                status: OrderStatus::New
                                                            }
                                                        );
                                                    }
                                                }
                                                log::info!("Grids enabled with levels on KuCoin: {}", grid_level.len());
                                            }
                                        },
                                        Trend::UpTrend | Trend::DownTrend => {
                                            if grid_active {
                                                for (id, order) in grid_orders.iter() {
                                                    let _ = self.cancel_order(req).await;
                                                    log::info!("Cancelled order at level: {} for id: {}", order.level, id);
                                                }
                                                grid_orders.clear();
                                                grid_active = false;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if topic.contains("order") {
                        if let Some(data) = val.get("data") {
                            self.handle_market_data_kc(&mut grid_orders, data, &req.symbol);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl RestClient for KuCoinAuth {
    async fn place_order(&self, req: &OrderReq) -> Result<String, anyhow::Error> {
        let body = json!({
            "clientOid": req.id.clone(),
            "symbol": req.symbol.clone(),
            "side": match req.side {
                Side::Buy => "Buy",
                Side::Sell => "Sell"
            },
            "size": req.size.to_string(),
            "price": req.price.to_string(),
            "type": req.type_.to_string(),
            "timeInForce": "GTC"
        });

        let url = "https://api.kucoin.com/api/v1/orders";
        let body_str = body.to_string();
        let now = Utc::now().timestamp_millis();
        let query_string = format!("{}{}{}{}", now, "POST", "/api/v1/orders", body_str);
        let sign = signature(self.cfg.secret_key.as_bytes(), &query_string).await;
        let response = self.http.post(format!("{}?{}&signature={}", url, query_string, sign))
            .header("X-MBX-APIKEY", self.cfg.api_key.clone()).send().await?;

        if response.status().is_success() {
            return Err(anyhow::anyhow!(format!(
                "Invalid response received from KuCoin exchange while placing order: {:?}", 
                response.text().await)
            ));
        }

        let res = response.json::<serde_json::Value>().await?;
        let res_ = res.to_string();
        Ok(res_)
    }

    async fn cancel_order(&self, req: &OrderReq) -> Result<String, anyhow::Error> {
        let url = "https://api.kucoin.com/api/v1/orders";
        let now = Utc::now().timestamp_millis();
        let query_string = format!("symbol={}&origClientOrderId={}&timestamp={}", req.symbol, req.id, now);
        let sign = signature(self.cfg.secret_key.as_bytes(), &query_string).await; 
        let response = self.http.delete(format!("{}?{}&signature={}", url, query_string, sign))
            .header("X-MBX-APIKEY", self.cfg.api_key.clone()).send().await?;
        
        if response.status().is_success() {
            return Err(anyhow::anyhow!(format!(
                "Invalid response received from KuCoin excahnge while cancelling order: {:?}", 
                response.text().await)
            ));
        }

        let res = response.json::<serde_json::Value>().await?;
        let res_ = res.to_string();
        Ok(res_)
    }
}
