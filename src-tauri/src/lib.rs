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
        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sku TEXT,
            barcode TEXT UNIQUE,
            name TEXT NOT NULL,
            category_id INTEGER,
            unit TEXT NOT NULL DEFAULT 'adet',
            buy_price REAL NOT NULL DEFAULT 0,
            sell_price REAL NOT NULL DEFAULT 0,
            min_stock_level REAL NOT NULL DEFAULT 0,
            current_stock REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(category_id) REFERENCES categories(id)
        );

        CREATE TABLE IF NOT EXISTS stock_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL,
            transaction_type TEXT NOT NULL,
            quantity REAL NOT NULL,
            unit_price REAL,
            reference_no TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(product_id) REFERENCES products(id)
        );

        CREATE INDEX IF NOT EXISTS idx_barcode ON products(barcode);
        CREATE INDEX IF NOT EXISTS idx_name ON products(name);
        CREATE INDEX IF NOT EXISTS idx_tx_product ON stock_transactions(product_id);
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
    pub category_id: Option<i64>,
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
    pub category_id: Option<i64>,
    pub unit: Option<String>,
    pub buy_price: Option<f64>,
    pub sell_price: Option<f64>,
    pub min_stock_level: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProduct {
    pub id: i64,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub category_id: Option<i64>,
    pub unit: Option<String>,
    pub buy_price: Option<f64>,
    pub sell_price: Option<f64>,
    pub min_stock_level: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub transaction_type: String,
    pub quantity: f64,
    pub unit_price: Option<f64>,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct StockValueReport {
    pub total_cost: f64,
    pub total_sell_value: f64,
    pub product_count: i64,
    pub low_stock_count: i64,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Veritabani hatasi: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Urun bulunamadi")]
    NotFound,
    #[error("Yetersiz stok")]
    InsufficientStock,
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn map_product(row: &rusqlite::Row) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        sku: row.get(1)?,
        barcode: row.get(2)?,
        name: row.get(3)?,
        category_id: row.get(4)?,
        unit: row.get(5)?,
        buy_price: row.get(6)?,
        sell_price: row.get(7)?,
        min_stock_level: row.get(8)?,
        current_stock: row.get(9)?,
    })
}

#[tauri::command]
fn list_products(search: Option<String>) -> Result<Vec<Product>, AppError> {
    let conn = DB.lock().unwrap();
    let (sql, params_vec): (String, Vec<String>) = match search {
        Some(s) if !s.trim().is_empty() => {
            let q = format!("%{}%", s.trim());
            (
                "SELECT id, sku, barcode, name, category_id, unit, buy_price, sell_price, min_stock_level, current_stock 
                 FROM products 
                 WHERE name LIKE ?1 OR barcode LIKE ?1 OR sku LIKE ?1 
                 ORDER BY name".to_string(),
                vec![q],
            )
        }
        _ => (
            "SELECT id, sku, barcode, name, category_id, unit, buy_price, sell_price, min_stock_level, current_stock 
             FROM products ORDER BY name".to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = if params_vec.is_empty() {
        stmt.query_map([], map_product)?
    } else {
        stmt.query_map(params![params_vec[0]], map_product)?
    };
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
fn get_product_by_barcode(barcode: String) -> Result<Product, AppError> {
    let conn = DB.lock().unwrap();
    conn.query_row(
        "SELECT id, sku, barcode, name, category_id, unit, buy_price, sell_price, min_stock_level, current_stock 
         FROM products WHERE barcode = ?1",
        params![barcode],
        map_product,
    )
    .map_err(|_| AppError::NotFound)
}

#[tauri::command]
fn create_product(p: NewProduct) -> Result<i64, AppError> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT INTO products (sku, barcode, name, category_id, unit, buy_price, sell_price, min_stock_level) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            p.sku,
            p.barcode,
            p.name,
            p.category_id,
            p.unit.unwrap_or_else(|| "adet".into()),
            p.buy_price.unwrap_or(0.0),
            p.sell_price.unwrap_or(0.0),
            p.min_stock_level.unwrap_or(0.0),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn update_product(p: UpdateProduct) -> Result<(), AppError> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "UPDATE products SET 
            sku = COALESCE(?1, sku),
            barcode = COALESCE(?2, barcode),
            name = COALESCE(?3, name),
            category_id = COALESCE(?4, category_id),
            unit = COALESCE(?5, unit),
            buy_price = COALESCE(?6, buy_price),
            sell_price = COALESCE(?7, sell_price),
            min_stock_level = COALESCE(?8, min_stock_level),
            updated_at = datetime('now')
         WHERE id = ?9",
        params![
            p.sku, p.barcode, p.name, p.category_id, p.unit,
            p.buy_price, p.sell_price, p.min_stock_level, p.id
        ],
    )?;
    Ok(())
}

#[tauri::command]
fn delete_product(id: i64) -> Result<(), AppError> {
    let conn = DB.lock().unwrap();
    conn.execute("DELETE FROM stock_transactions WHERE product_id = ?1", params![id])?;
    conn.execute("DELETE FROM products WHERE id = ?1", params![id])?;
    Ok(())
}

#[tauri::command]
fn stock_in(product_id: i64, quantity: f64, unit_price: Option<f64>, reference_no: Option<String>, notes: Option<String>) -> Result<(), AppError> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "UPDATE products SET current_stock = current_stock + ?1, updated_at = datetime('now') WHERE id = ?2",
        params![quantity, product_id],
    )?;
    if let Some(price) = unit_price {
        conn.execute(
            "UPDATE products SET buy_price = ?1 WHERE id = ?2",
            params![price, product_id],
        )?;
    }
    conn.execute(
        "INSERT INTO stock_transactions (product_id, transaction_type, quantity, unit_price, reference_no, notes) 
         VALUES (?1, 'in', ?2, ?3, ?4, ?5)",
        params![product_id, quantity, unit_price, reference_no, notes],
    )?;
    Ok(())
}

#[tauri::command]
fn stock_out(product_id: i64, quantity: f64, reference_no: Option<String>, notes: Option<String>) -> Result<(), AppError> {
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
        "INSERT INTO stock_transactions (product_id, transaction_type, quantity, reference_no, notes) 
         VALUES (?1, 'out', ?2, ?3, ?4)",
        params![product_id, quantity, reference_no, notes],
    )?;
    Ok(())
}

#[tauri::command]
fn adjust_stock(product_id: i64, new_quantity: f64, notes: Option<String>) -> Result<(), AppError> {
    let conn = DB.lock().unwrap();
    let current: f64 = conn.query_row(
        "SELECT current_stock FROM products WHERE id = ?1",
        params![product_id],
        |row| row.get(0),
    )?;
    let diff = new_quantity - current;
    conn.execute(
        "UPDATE products SET current_stock = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_quantity, product_id],
    )?;
    conn.execute(
        "INSERT INTO stock_transactions (product_id, transaction_type, quantity, notes) 
         VALUES (?1, 'adjustment', ?2, ?3)",
        params![product_id, diff, notes],
    )?;
    Ok(())
}

#[tauri::command]
fn low_stock_products() -> Result<Vec<Product>, AppError> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sku, barcode, name, category_id, unit, buy_price, sell_price, min_stock_level, current_stock 
         FROM products WHERE current_stock <= min_stock_level ORDER BY current_stock"
    )?;
    let rows = stmt.query_map([], map_product)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
fn list_transactions(limit: Option<i64>) -> Result<Vec<Transaction>, AppError> {
    let conn = DB.lock().unwrap();
    let lim = limit.unwrap_or(50);
    let mut stmt = conn.prepare(
        "SELECT t.id, t.product_id, p.name, t.transaction_type, t.quantity, t.unit_price, t.reference_no, t.notes, t.created_at
         FROM stock_transactions t
         JOIN products p ON p.id = t.product_id
         ORDER BY t.created_at DESC
         LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![lim], |row| {
        Ok(Transaction {
            id: row.get(0)?,
            product_id: row.get(1)?,
            product_name: row.get(2)?,
            transaction_type: row.get(3)?,
            quantity: row.get(4)?,
            unit_price: row.get(5)?,
            reference_no: row.get(6)?,
            notes: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
fn stock_value_report() -> Result<StockValueReport, AppError> {
    let conn = DB.lock().unwrap();
    let total_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(current_stock * buy_price), 0) FROM products",
        [],
        |row| row.get(0),
    )?;
    let total_sell: f64 = conn.query_row(
        "SELECT COALESCE(SUM(current_stock * sell_price), 0) FROM products",
        [],
        |row| row.get(0),
    )?;
    let product_count: i64 = conn.query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))?;
    let low_stock_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products WHERE current_stock <= min_stock_level",
        [],
        |row| row.get(0),
    )?;
    Ok(StockValueReport {
        total_cost,
        total_sell_value: total_sell,
        product_count,
        low_stock_count,
    })
}

#[tauri::command]
fn export_products_csv() -> Result<String, AppError> {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sku, barcode, name, unit, buy_price, sell_price, min_stock_level, current_stock FROM products ORDER BY name"
    )?;

    let mut csv = String::from("id,sku,barcode,name,unit,buy_price,sell_price,min_stock_level,current_stock\n");

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, f64>(7)?,
            row.get::<_, f64>(8)?,
        ))
    })?;

    for row in rows {
        let (id, sku, barcode, name, unit, buy, sell, min_s, stock) = row?;
        let safe_name = name.replace('"', "'");
        csv.push_str(&format!(
            "{},{},{},\"{}\",{},{},{},{},{}\n",
            id, sku, barcode, safe_name, unit, buy, sell, min_s, stock
        ));
    }

    let mut dir = PathBuf::from("exports");
    fs::create_dir_all(&dir).map_err(|e| AppError::Other(format!("Klasor olusturulamadi: {}", e)))?;

    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let filename = format!("stok-urunler-{}.csv", ts);
    dir.push(&filename);

    // UTF-8 BOM so Excel opens Turkish characters correctly
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(csv.as_bytes());

    fs::write(&dir, &bytes).map_err(|e| AppError::Other(format!("Dosya yazilamadi: {}", e)))?;

    Ok(dir.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_products,
            get_product_by_barcode,
            create_product,
            update_product,
            delete_product,
            stock_in,
            stock_out,
            adjust_stock,
            low_stock_products,
            list_transactions,
            stock_value_report,
            export_products_csv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
