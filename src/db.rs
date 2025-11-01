use anyhow::Context;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use anyhow::Result;
use tracing::info;
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

        Ok(Self { pool })
    }

    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trades (
                id SERIAL PRIMARY KEY,
                trade_id VARCHAR(255) UNIQUE NOT NULL,
                symbol VARCHAR(50) NOT NULL,
                side VARCHAR(10) NOT NULL,
                entry_price DECIMAL(20, 8) NOT NULL,
                quantity DECIMAL(20, 8) NOT NULL,
                stop_loss DECIMAL(20, 8),
                take_profit DECIMAL(20, 8),
                opened_at TIMESTAMPTZ NOT NULL,
                closed_at TIMESTAMPTZ,
                exit_price DECIMAL(20, 8),
                pnl DECIMAL(20, 8),
                status VARCHAR(20) NOT NULL,
                manual BOOLEAN NOT NULL DEFAULT FALSE
            );

            CREATE TABLE IF NOT EXISTS signals (
                id SERIAL PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                symbol VARCHAR(50) NOT NULL,
                action VARCHAR(10) NOT NULL,
                price DECIMAL(20, 8) NOT NULL,
                confidence DECIMAL(5, 4) NOT NULL,
                trend VARCHAR(20) NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol);
            CREATE INDEX IF NOT EXISTS idx_trades_status ON trades(status);
            CREATE INDEX IF NOT EXISTS idx_signals_timestamp ON signals(timestamp);
            "#
        ).execute(&self.pool).await?;

        info!("Database schema initialized!");

        Ok(())
    }

    pub async fn save_order(&self, position: &Position, manual: bool) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO trades (trade_id, symbol, position_side, entry_price, quantity
                                stop_loss, take_profit, opened_at, status, manual)
            VAlUE ($1, $2, $3, $4, $5, $6, $7, $8, 'open', $9)               
            "#
        )
        .bind(&position.id)
        .bind(&position.symbol)
        .bind(format!("{:?}", position.position_side))
        .bind(&position.entry_price)
        .bind(&position.size)
        .bind(&position.stop_loss)
        .bind(&position.take_profit)
        .bind(&position.opened_at)
        .bind(DateTime::<Utc>::from_timestamp(position.opened_at, 0))
        .bind(manual)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn close_order(&self, trade_id: &str, exit_price: Decimal, pnl: Decimal) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE trades
            SET closed_at = $1, exit_price = $2, pnl = $3, status = 'closed'
            WHERE trade_id = $4
            "#
        )
        .bind(Utc::now())
        .bind(exit_price)
        .bind(pnl)
        .bind(trade_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_signal(&self, signal: Signal) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO signal (timestamp, symbol, action, price, confidence, trend)
            VALUE ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(&signal.timestamp)
        .bind(&signal.symbol)
        .bind(format!("{:?}", signal.action))
        .bind(&signal.price)
        .bind(&signal.confidence)
        .bind(format!("{:?}", signal.trend))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_open_orders(&self) -> Result<Vec<Position>> {
        let query = sqlx::query_as::<_, (String, String, String, Decimal, Decimal, Decimal, Decimal, DateTime<Utc>)>(
            r#"
            SELECT trade_id, symbol, position_side, entry_price, quantity, stop_loss, take_profit, opened_at
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
            opened_at: row.7.timestamp()
        }).collect();

        Ok(position)
    }
}
