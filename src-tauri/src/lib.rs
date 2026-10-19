use once_cell::sync::Lazy;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;

static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let conn = Connection::open("stock.db").expect("DB acilamadi");
    init_db(&conn).expect("Migration hatasi");
    Mutex::new(conn)
});

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sku TEXT,
            barcode TEXT UNIQUE,
            name TEXT NOT NULL,
            unit TEXT NOT NULL DEFAULT 'adet',
            buy_price REAL NOT NULL DEFAULT 0,
            sell_price REAL NOT NULL DEFAULT 0,
            min_stock_level REAL NOT NULL DEFAULT 0,
            current_stock REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS stock_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL,
            transaction_type TEXT NOT NULL,
            quantity REAL NOT NULL,
            unit_price REAL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(product_id) REFERENCES products(id)
        );

        CREATE INDEX IF NOT EXISTS idx_barcode ON products(barcode);
        "#,
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: i64,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub name: String,
    pub unit: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub min_stock_level: f64,
    pub current_stock: f64,
}

#[derive(Debug, Deserialize)]
pub struct NewProduct {
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub name: String,
    pub unit: Option<String>,
    pub buy_price: Option<f64>,
    pub sell_price: Option<f64>,
    pub min_stock_level: Option<f64>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Veritabani hatasi: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Urun bulunamadi")]
    NotFound,
    #[error("Yetersiz stok")]
    InsufficientStock,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[tauri::command]
fn list_products() -> Result<Vec<Product>, AppError> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sku, barcode, name, unit, buy_price, sell_price, min_stock_level, current_stock FROM products ORDER BY name"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Product {
            id: row.get(0)?,
            sku: row.get(1)?,
            barcode: row.get(2)?,
            name: row.get(3)?,
            unit: row.get(4)?,
            buy_price: row.get(5)?,
            sell_price: row.get(6)?,
            min_stock_level: row.get(7)?,
            current_stock: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
fn get_product_by_barcode(barcode: String) -> Result<Product, AppError> {
    let conn = DB.lock().unwrap();
    conn.query_row(
        "SELECT id, sku, barcode, name, unit, buy_price, sell_price, min_stock_level, current_stock FROM products WHERE barcode = ?1",
        params![barcode],
        |row| {
            Ok(Product {
                id: row.get(0)?,
                sku: row.get(1)?,
                barcode: row.get(2)?,
                name: row.get(3)?,
                unit: row.get(4)?,
                buy_price: row.get(5)?,
                sell_price: row.get(6)?,
                min_stock_level: row.get(7)?,
                current_stock: row.get(8)?,
            })
        },
    )
    .map_err(|_| AppError::NotFound)
}

#[tauri::command]
fn create_product(p: NewProduct) -> Result<i64, AppError> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT INTO products (sku, barcode, name, unit, buy_price, sell_price, min_stock_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            p.sku,
            p.barcode,
            p.name,
            p.unit.unwrap_or_else(|| "adet".into()),
            p.buy_price.unwrap_or(0.0),
            p.sell_price.unwrap_or(0.0),
            p.min_stock_level.unwrap_or(0.0),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn stock_in(product_id: i64, quantity: f64, unit_price: Option<f64>, notes: Option<String>) -> Result<(), AppError> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "UPDATE products SET current_stock = current_stock + ?1, updated_at = datetime('now') WHERE id = ?2",
        params![quantity, product_id],
    )?;
    conn.execute(
        "INSERT INTO stock_transactions (product_id, transaction_type, quantity, unit_price, notes) VALUES (?1, 'in', ?2, ?3, ?4)",
        params![product_id, quantity, unit_price, notes],
    )?;
    Ok(())
}

#[tauri::command]
fn stock_out(product_id: i64, quantity: f64, notes: Option<String>) -> Result<(), AppError> {
    let conn = DB.lock().unwrap();
    let current: f64 = conn.query_row(
        "SELECT current_stock FROM products WHERE id = ?1",
        params![product_id],
        |row| row.get(0),
    )?;
    if current < quantity {
        return Err(AppError::InsufficientStock);
    }
    conn.execute(
        "UPDATE products SET current_stock = current_stock - ?1, updated_at = datetime('now') WHERE id = ?2",
        params![quantity, product_id],
    )?;
    conn.execute(
        "INSERT INTO stock_transactions (product_id, transaction_type, quantity, notes) VALUES (?1, 'out', ?2, ?3)",
        params![product_id, quantity, notes],
    )?;
    Ok(())
}

#[tauri::command]
fn low_stock_products() -> Result<Vec<Product>, AppError> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sku, barcode, name, unit, buy_price, sell_price, min_stock_level, current_stock FROM products WHERE current_stock <= min_stock_level ORDER BY current_stock"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Product {
            id: row.get(0)?,
            sku: row.get(1)?,
            barcode: row.get(2)?,
            name: row.get(3)?,
            unit: row.get(4)?,
            buy_price: row.get(5)?,
            sell_price: row.get(6)?,
            min_stock_level: row.get(7)?,
            current_stock: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_products,
            get_product_by_barcode,
            create_product,
            stock_in,
            stock_out,
            low_stock_products
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
