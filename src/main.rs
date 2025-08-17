use std::{collections::HashMap, env, vec};
use chrono::Utc;
use tokio::{sync::mpsc, task};
use uuid::Uuid;
use crate::{data::{Exchange, OrderReq, OrderStatus, Side, TopOfBook}, engine::Engine,
    exchange::{binance_auth::Binance, config::{Config, Exchangecfg}, 
    kucoin_auth::KuCoin, StreamBook}, strategy::market_making::MM
};

mod tests;
mod exchange;
mod utils;
mod data;
mod strategy;
mod engine;
mod ws_stream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    
    let cfg = Config {
        kucoin: Exchangecfg {
            api_key: env::var("API_KEY").expect("KuCoin API key is not set!"),
            secret_key: env::var("SECRET_KEY").expect("KuCoin secret key is not set!")
        },
        binance: Exchangecfg {
            api_key: env::var("API_KEY1").expect("Binance API key is not set!"),
            secret_key: env::var("SECRET_KEY1").expect("Binance secret key is not set!")
        },
        paper: true
    };

    let kucoin_symbol = "ETH-USDT";
    let (tx, mut rx) = mpsc::unbounded_channel::<TopOfBook>();
    let (order_tx, mut order_rx) = mpsc::unbounded_channel::<OrderReq>();
    let cfg1 = cfg.kucoin.clone();
    let cfg2 = cfg.binance.clone();
    let client1 = KuCoin::new(cfg.kucoin.clone(), kucoin_symbol).await.unwrap();
    let client2 = Binance::new(cfg.binance.clone()).await.unwrap();
    let mut engine1 = Engine::new(client1, cfg.paper);
    let mut engine2 = Engine::new(client2, cfg.paper);
    let mut mm = MM::new();

    let mut order_status: HashMap<String, OrderStatus> = HashMap::new();

    {
        let cli_tx = order_tx.clone();
        let symbols = vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()];

        task::spawn(async move {
            use dialoguer::{theme::ColorfulTheme, Select, Input};
            loop {
                let menu = vec!["Place Order", "Quit"];
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .items(&menu)
                    .default(0)
                    .interact()
                    .unwrap();

                if menu[selection] == "Quit" {
                    println!("Existing CLI...");
                    break;
                }

                let exchanges = vec!["KuCoin", "Binance"];
                let excg_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Choosing exchange...")
                    .items(&exchanges)
                    .default(0)
                    .interact()
                    .unwrap();
                let exchange_ = if excg_selection == 0 {
                    Exchange::KuCoin
                }
                else {
                    Exchange::Binance
                };

                let sym_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Choosing asset...")
                    .items(&symbols)
                    .default(0)
                    .interact()
                    .unwrap();
                let symbol = symbols[sym_selection].clone();

                let price = Input::new()
                    .with_prompt("Enter price...")
                    .interact_text()
                    .unwrap();
                let quantity = Input::new()
                    .with_prompt("Enter quantity...")
                    .interact_text()
                    .unwrap();

                let sides = vec!["Buy", "Sell"];
                let side_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Choosing side...")
                    .items(&sides)
                    .default(0)
                    .interact()
                    .unwrap();
                let side = if side_selection == 0 {
                    Side::Buy
                }
                else {
                    Side::Sell
                };

                let order = OrderReq {
                    id: Uuid::new_v4().to_string(),
                    exchange: exchange_,
                    symbol,
                    price,
                    quantity,
                    side,
                    timestamp: Utc::now().timestamp_millis()
                };

                if let Err(e) = cli_tx.send(order) {
                    eprintln!("Failed to send the error in the main loop: {}", e);
                }
                else {
                    println!("User order sent(queued)");
                }
            }
        });
    }

    task::spawn(async move {
        let mut ws1 = KuCoin::new(cfg1, kucoin_symbol).await.unwrap();
        while let Ok(tob) = ws1.next_tob().await {
            let _ = tx.send(tob);
        }
        let mut ws2 = Binance::new(cfg2).await.unwrap();
        while let Ok(tob) = ws2.next_tob().await {
            let _ = tx.send(tob);
        }
    });

    loop {
        tokio::select! {
            Some(tob_) = rx.recv() => {
                tracing::info!("Received top of the orderbook: {:?}", tob_);
                println!("Symbol: {}", tob_.symbol);
                println!("Bid price from top of the orderbook: {:.2}", tob_.bid);
                println!("Ask price from top of the orderbook: {:.2}", tob_.ask);
                println!("Timestamp: {}", tob_.timestamp);
                
                if let Some(order) = mm.decide(vec![tob_.bid], tob_.exchange.clone(), &tob_) {
                    tracing::info!("Generate order: {:?}", order);
                    println!("Order ID: {}", order.id);
                    println!("Price: {:.2}", order.price);
                    println!("Quantity: {:.4}", order.quantity);
                    println!("Order side: {:?}", order.side);

                    if matches!(tob_.exchange, Exchange::KuCoin) {
                        engine1.handle(&order).await.unwrap();
                        println!("Handling order on KuCoin exchange...");
                    }
                    else if matches!(tob_.exchange, Exchange::Binance) {
                        engine2.handle(&order).await.unwrap();
                        println!("Handling order on Binance exchange...");
                    }
                }
            }

            Some(user_order) = order_rx.recv() => {
                tracing::info!("User placed order: {:?}", user_order);
                order_status.insert(user_order.id.to_string(), OrderStatus::New);
                println!("Order {} status {:?}", user_order.id.clone(), order_status);

                order_status.insert(user_order.id.clone(), OrderStatus::Sent);
                println!("Order {} status: {:?}", user_order.id.clone(), order_status);

                let result = match user_order.exchange {
                    Exchange::KuCoin => engine1.handle(&user_order).await,
                    Exchange::Binance => engine2.handle(&user_order).await
                };

                match result {
                    Ok(_) => {
                        order_status.insert(user_order.id.to_string(), OrderStatus::Filled);
                        println!("Order {} status: {:?}", user_order.id.clone(), order_status);
                    },                                            
                    Err(e) => {
                        order_status.insert(user_order.id.to_string(), OrderStatus::Rejected);
                        println!("Order {} rejected with error: {} and status: {:?}", user_order.id.clone(), e, order_status);
                    },
                }            
            },
                
            else => {
                break;
            }
        }
    }
    Ok(())
}
