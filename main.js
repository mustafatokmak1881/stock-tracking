// Stok Takip MVP - localStorage backed
const STORAGE_KEY %3D 'stock_products'%3B

function loadProducts() {
  const raw %3D localStorage.getItem(STORAGE_KEY)%3B
  return raw %3F JSON.parse(raw) %3A []%3B
}

function saveProducts(products) {
  localStorage.setItem(STORAGE_KEY%2C JSON.stringify(products))%3B
}

function generateId() {
  return Date.now() %2B Math.floor(Math.random() * 1000)%3B
}

function render() {
  const products %3D loadProducts()%3B
  const tbody %3D document.getElementById('product-tbody')%3B
  if (!tbody) return%3B
  tbody.innerHTML %3D products.map(p %3D> `
    <tr>
      <td>%24{p.barcode || '-'}<%2Ftd>
      <td>%24{p.name}<%2Ftd>
      <td>%24{p.current_stock} %24{p.unit}<%2Ftd>
      <td>%24{p.sell_price}<%2Ftd>
      <td>
        <button onclick%3D"stockIn(%24{p.id})">%2B<%2Fbutton>
        <button onclick%3D"stockOut(%24{p.id})">-<%2Fbutton>
      <%2Ftd>
    <%2Ftr>
  `).join('')%3B

  const low %3D products.filter(p %3D> p.current_stock <%3D p.min_stock_level)%3B
  const alerts %3D document.getElementById('low-stock-alerts')%3B
  if (alerts) {
    alerts.innerHTML %3D low.length %3D%3D%3D 0 
      %3F '<p>Kritik stok yok<%2Fp>' 
      %3A low.map(p %3D> `<div class%3D"card alert-danger">%24{p.name} - Kalan%3A %24{p.current_stock} %24{p.unit}<%2Fdiv>`).join('')%3B
  }
}

function addProduct() {
  const name %3D document.getElementById('p-name').value.trim()%3B
  const barcode %3D document.getElementById('p-barcode').value.trim()%3B
  const unit %3D document.getElementById('p-unit').value || 'adet'%3B
  const buy %3D parseFloat(document.getElementById('p-buy').value) || 0%3B
  const sell %3D parseFloat(document.getElementById('p-sell').value) || 0%3B
  const min %3D parseFloat(document.getElementById('p-min').value) || 0%3B
  if (!name) return alert('Ürün adı gerekli')%3B
  const products %3D loadProducts()%3B
  products.push({
    id%3A generateId()%2C
    barcode%2C
    name%2C
    unit%2C
    buy_price%3A buy%2C
    sell_price%3A sell%2C
    min_stock_level%3A min%2C
    current_stock%3A 0
  })%3B
  saveProducts(products)%3B
  render()%3B
  document.getElementById('p-name').value %3D ''%3B
  document.getElementById('p-barcode').value %3D ''%3B
}

function stockIn(id) {
  const qty %3D parseFloat(prompt('Giriş miktarı'%2C '1'))%3B
  if (!qty || qty <%3D 0) return%3B
  const products %3D loadProducts()%3B
  const p %3D products.find(x %3D> x.id %3D%3D%3D id)%3B
  if (p) { p.current_stock %2B%3D qty%3B saveProducts(products)%3B render()%3B }
}

function stockOut(id) {
  const qty %3D parseFloat(prompt('Çıkış miktarı'%2C '1'))%3B
  if (!qty || qty <%3D 0) return%3B
  const products %3D loadProducts()%3B
  const p %3D products.find(x %3D> x.id %3D%3D%3D id)%3B
  if (p) {
    if (p.current_stock < qty) return alert('Yetersiz stok')%3B
    p.current_stock -%3D qty%3B
    saveProducts(products)%3B
    render()%3B
  }
}

document.addEventListener('DOMContentLoaded'%2C render)%3B