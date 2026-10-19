# Stok Takip MVP (Tauri + Rust + SQLite + Vanilla)

## Çalıştırma

```bash
cd /root/stock-tracking-mvp
npm install
npm run dev
```

## Özellikler
- Ürün ekleme / listeleme
- Barkod desteği
- Stok giriş / çıkış
- Kritik stok uyarıları
- SQLite veritabanı (stock.db)

## Komutlar (Rust)
- list_products
- get_product_by_barcode
- create_product
- stock_in
- stock_out
- low_stock_products
