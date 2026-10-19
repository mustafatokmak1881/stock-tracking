# Stok Takip - Tam MVP

Tauri + Rust + SQLite + Vanilla JS

## Özellikler

### Faz 1-2
- SQLite veritabanı (products, categories, stock_transactions)
- Ürün ekleme / düzenleme / silme
- Barkod ile arama
- Anlık arama (ad, barkod, SKU)

### Faz 3
- Stok giriş (alış fiyatı güncelleme destekli)
- Stok çıkış (yetersiz stok kontrolü)
- Sayım / düzeltme (adjustment)

### Faz 4
- Kritik stok uyarıları
- Stok değerleme raporu (maliyet + potansiyel satış)
- Hareket geçmişi (audit log)
- CSV export

### Faz 5
- Koyu / Açık tema
- Klavye kısayolları (F2, F4, F5)
- Modern, hızlı arayüz

## Çalıştırma

```bash
cd /root/stock-tracking-mvp
npm install
npm run dev
```

## Kısayollar
- F2 → Yeni ürün
- F4 → Barkod odaklı dashboard
- F5 → Yenile
