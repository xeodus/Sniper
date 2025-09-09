use std::{env, time::Duration};
use clap::{arg, Parser};
use chrono::Utc;
use dotenv::dotenv;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{data::{BotState, Exchange, GridOrder, OrderReq, OrderStatus, Side, Trend}, 
    exchange::{binance_auth::Binance, config::{Config, Exchangecfg},
    kucoin_auth::KuCoin, RestClient}, store::OrderStore};

mod engine;
mod data;
mod utils;
mod exchange;
mod store;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value="ETH-USDT")]
    symbol: String,
    #[arg(long, default_value="1min")]
    timeframe: String,
    #[arg(long, default_value="orders.db")]
    db: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();
    let cfg = Config {
        kucoin: Exchangecfg {
            api_key: env::var("KUCOIN_API_KEY").expect("KuCoin API key not found!"),
            secret_key: env::var("KUCOIN_SECRET_KEY").expect("KuCoin secret key not found!")
        },
        binance: Exchangecfg {
            api_key: env::var("BINANCE_API_KEY").expect("Binance API key not found!"),
            secret_key: env::var("BINANCE_SECRET_KEY").expect("Binance secret key not found!")
        },
        paper: true
    };

    let args = Args::parse();
    log::info!("Starting grid bot for {} @ {}", args.symbol, args.timeframe);

    let exchange = Exchange::Binance;
    let side = Side::Buy;

    let req = OrderReq {
        id: Uuid::new_v4().to_string(),
        exchange: match exchange {
            Exchange::Binance => Exchange::Binance,
            Exchange::KuCoin => Exchange::KuCoin
        },
        symbol: args.symbol.clone(),
        type_: "limit".to_string(),
        price: 0.0,
        quantity: 0.001,
        side: match side {
            Side::Buy => Side::Buy,
            Side::Sell => Side::Sell
        },
        timestamp: Utc::now().timestamp_millis()
    };

    let grid = GridOrder {
        client_oid: Uuid::new_v4().to_string(),
        symbol: args.symbol.clone(),
        level: req.price,
        side: match side {
            Side::Buy => Side::Buy,
            Side::Sell => Side::Sell
        },
        quantity: 0.001,
        active: true,
        status: OrderStatus::New
    };

    let (tx, _) = watch::channel(BotState {
        trend: Trend::SideChop,
        open_orders: Vec::new(),
        pnl: 0.0
    });

    tokio::spawn(async move {
        loop {
            let mut store = OrderStore::init_db(&args.db).unwrap();
            log::info!("SQLite persistance ready: {:?}", args.db);

            let open_orders = OrderStore::db_load_orders(&store.conn).unwrap();
            log::info!("Restored {} open orders from DB", open_orders.len());

            let mut kc = KuCoin::new(cfg.kucoin);
            let _ = kc.ws_connect(&req).await;
            log::info!("Successfully connected to KuCoin WebSocket!");
            let place_order_kucoin = kc.place_order(&req).await;
            log::info!("Placed order on KuCoin: {:?}", place_order_kucoin);
            let cancel_order_kucoin = kc.cancel_order(&req).await;
            log::error!("Cancelled order on KuCoin: {:?}", cancel_order_kucoin);

            let mut bn = Binance::new(cfg.binance);
            let _ = bn.ws_connect(&req).await;
            log::info!("Successfully connected to Binance WebSocket!");
            let place_order_binance = bn.place_order(&req).await;
            log::info!("Placed order on Binance: {:?}", place_order_binance);
            let cancel_order_binance = bn.cancel_order(&req).await;
            log::info!("Cancelled order on Binance: {:?}", cancel_order_binance);

            let mut st = tx.borrow().clone();

            if matches!(st.trend, Trend::UpTrend) {
                st.open_orders = vec![GridOrder {
                    client_oid: Uuid::new_v4().to_string(),
                    symbol: args.symbol.clone(),
                    level: req.price,
                    side: Side::Buy,
                    quantity: 0.001,
                    active: true,
                    status: OrderStatus::New
                }];
            }
            else if matches!(st.trend, Trend::DownTrend) {
                st.open_orders = vec![GridOrder {
                    client_oid: Uuid::new_v4().to_string(),
                    symbol: args.symbol.clone(),
                    level:req.price,
                    side: Side::Sell,
                    quantity: 0.001,
                    active: true,
                    status: OrderStatus::New
                }];
            }
            else if matches!(st.trend, Trend::SideChop) {
                st.open_orders = vec![GridOrder {
                    client_oid: Uuid::new_v4().to_string(),
                    symbol: args.symbol.clone(),
                    level: req.price,
                    side,
                    quantity: 0.001,
                    active: true,
                    status: OrderStatus::New
                }];
            }

            let saved_order_on_db = store.db_save_orders(&grid).unwrap();
            log::info!("Executed and saved order on DB: {:?}", saved_order_on_db);

            let order_update = store.db_update_status(&grid).unwrap();
            log::info!("Order updates: {:?}", order_update);

            
            tokio::time::sleep(Duration::from_secs(5)).await;
            break;
        }
    });
    Ok(())
}
