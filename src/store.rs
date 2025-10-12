use rusqlite::{params, Connection, Result};
use crate::data::{GridOrder, OrderStatus, Side};

pub struct OrderStore {
    pub conn: Connection
}

impl OrderStore {
    pub fn init_db(path: &str) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS grid_orders (
                client_oid TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                level REAL NOT NULL,
                side TEXT NOT NULL,
                quantity REAL NOT NULL,
                active BOOLEAN NOT NULL DEFAULT 1,
                status TEXT NOT NULL DEFAULT 'new',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        )?;
        Ok(Self { conn: connection })
    }

    pub fn db_save_orders(&mut self, o: &GridOrder) -> Result<()> {
        let side = match o.side {
            Side::Buy => "Buy",
            Side::Sell => "Sell"
        };
        let status = match o.status {
            OrderStatus::Filled => "filled",
            OrderStatus::New => "new",
            OrderStatus::Rejected => "rejected"
        };
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT OR REPLACE INTO grid_orders (client_oid, symbol, level, side, quantity, active, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", 
            params![
                o.client_oid.clone(), 
                o.symbol.clone(), 
                o.level, 
                side, 
                o.size, 
                o.active, 
                status,
                now,
                now
            ]
        )?;
        Ok(())
    }

    pub async fn db_update_status(&mut self, o: &GridOrder) -> Result<()> {
        let status_ = match o.status {
            OrderStatus::New => "new",
            OrderStatus::Filled => "filled",
            OrderStatus::Rejected => "rejected"
        };
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "UPDATE grid_orders SET status = ?2, updated_at = ?3 WHERE client_oid = ?1", 
            params![o.client_oid, status_, now]
        )?;
        Ok(())
    }

    pub fn db_load_orders(conn: &Connection) -> Result<Vec<GridOrder>> {
        let mut stmt = conn.prepare("SELECT 
            client_oid, symbol, level, side, quantity, active, status FROM grid_orders WHERE active = 1 AND status IN ('new', 'filled')")?;
        let mut rows = stmt.query([])?;
        let mut v = Vec::new();
        while let Some(r) = rows.next()? {
            let side_init: String = r.get(3)?;
            let side = match side_init.as_str() {
                "Buy" => Side::Buy,
                "Sell" => Side::Sell,
                &_ => {
                    log::warn!("Unspecified side detected, marking as Buy!");
                    Side::Buy
                }
            };
            let status_init: String = r.get(6)?;
            let status = match status_init.as_str() {
                "new" => OrderStatus::New,
                "filled" => OrderStatus::Filled,
                "rejected" => OrderStatus::Rejected,
                &_ => {
                    log::warn!("Unknown status detected in DB, marking as rejected..");
                    OrderStatus::Rejected
                }
            };

            v.push(GridOrder {
                client_oid: r.get(0)?,
                symbol: r.get(1)?,
                level: r.get(2)?,
                side,
                active: r.get(5)?,
                size: r.get(4)?,
                status
            });
        }
        Ok(v)
    }
}
