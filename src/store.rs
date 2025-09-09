use rusqlite::{params, Connection, Result};
use crate::data::{GridOrder, OrderStatus, Side};

pub struct OrderStore {
    pub conn: Connection
}

impl OrderStore {
    pub fn init_db(path: &str) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS orders (
                client_oid TEXT PRIMARY KEY,
                symbol TEXT,
                side TEXT,
                price REAL,
                status TEXT
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

        self.conn.execute("INSERT OR REPLACE INTO orders (client_oid, symbol, side, price, quantity, active, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)", 
            params![o.client_oid.clone(), o.symbol.clone(), side, o.level, o.quantity, status])?;
        Ok(())
    }

    pub fn db_update_status(&mut self, o: &GridOrder) -> Result<()> {
        let status_ = match o.status {
            OrderStatus::New => "new",
            OrderStatus::Filled => "filled",
            OrderStatus::Rejected => "rejected"
        };
        self.conn.execute("UPDATE orders SET status = ?2 WHERE client_oid = ?1", params![o.client_oid, status_])?;
        Ok(())
    }

    pub fn db_load_orders(conn: &Connection) -> Result<Vec<GridOrder>> {
        let mut stmt = conn.prepare("SELECT 
            client_oid, symbol, price, side, quantity, active, status FROM grid_orders where status = 'open'")?;
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
            let status_init: String = r.get(5)?;
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
                active: r.get(4)?,
                quantity: r.get(5)?,
                status
            });
        }
        Ok(v)
    }
}
