use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use anyhow::Result;
use crate::data::{Position, PositionSide, Signal};

pub struct Database {
    pub pool: PgPool
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .context("Failed to connect to database!")?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    } 

    pub async fn save_order(&self, position: &Position, manual: bool) -> Result<()> {
        let opened = position.opened_at;
        let opened_at = Utc.timestamp_opt(opened, 0).single().unwrap();
        let closed = position.closed_at;
        let closed_at = Utc.timestamp_opt(closed, 0).single().unwrap();

        sqlx::query!(
            r#"
            INSERT INTO trades (trade_id, symbol, side, entry_price, quantity,
            stop_loss, take_profit, opened_at, closed_at, exit_price, pnl, status, manual)
            VAlUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)               
            "#,
            position.id, position.symbol, format!("{:?}", position.position_side), position.entry_price,
            position.size, position.stop_loss, position.take_profit, opened_at,
            closed_at, position.exit_price, position.pnl, "open", manual
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn close_order(&self, trade_id: &str, exit_price: Decimal, pnl: Decimal) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE trades
            SET closed_at = $1, exit_price = $2, pnl = $3, status = 'closed'
            WHERE trade_id = $4
            "#,
            now, exit_price, pnl, trade_id
        ) 
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_signal(&self, signal: Signal) -> Result<()> {
        let ts = signal.timestamp;
        let timestamp: DateTime<Utc> = Utc.timestamp_opt(ts, 0).single().unwrap();

        sqlx::query!(
            r#"
            INSERT INTO signals (id, timestamp, symbol, action, price, confidence, trend)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            signal.id, timestamp, signal.symbol, format!("{:?}", signal.action),
            signal.price, signal.confidence, format!("{:?}", signal.trend) 
        ) 
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_open_orders(&self) -> Result<Vec<Position>> {
        let query = sqlx::query_as::<_, (String, String, String, Decimal, Decimal, Decimal, 
            Decimal, DateTime<Utc>, DateTime<Utc>, Decimal, Decimal)>
        (
            r#"
            SELECT trade_id, symbol, side, entry_price, quantity, 
            stop_loss, take_profit, opened_at, closed_at, exit_price, pnl
            FROM trades WHERE status = 'open'
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let position = query.into_iter().map(|row| Position {
            id: row.0,
            symbol: row.1,
            position_side: if row.2 == "Long" { PositionSide::Long } else { PositionSide::Short },
            entry_price: row.3,
            size: row.4,
            stop_loss: row.5,
            take_profit: row.6,
            opened_at: row.7.timestamp(),
            closed_at: row.8.timestamp(),
            exit_price: row.9,
            pnl: row.10
        }).collect();

        Ok(position)
    }
}
