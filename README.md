# Axum Playground 🦀

Playground sederhana untuk belajar **Axum** (web framework Rust) dengan **SeaORM** + **PostgreSQL**.
Sengaja dibuat ringkas (tidak di-_overengineering_) supaya enak dipakai belajar.

## Fitur

| Fitur | Keterangan |
|-------|------------|
| 🔐 **Auth JWT** | Register & login, token JWT (HS256) via header `Authorization: Bearer <token>` |
| 👮 **RBAC sederhana** | Dua role: `admin` & `staff`. Dibuat lewat extractor `AuthUser` / `AdminUser` |
| 👥 **Account Management** | CRUD user (khusus admin): list, detail, buat, ubah role/status, hapus |
| 🙋 **Profile** | User login bisa lihat & ubah data dirinya sendiri (nama, email, password) |
| 📊 **Dashboard Stok** | Ringkasan: total produk, total qty, nilai stok, stok menipis/habis |
| 📦 **Menu Stok (produk)** | CRUD produk + penyesuaian stok (tambah/kurang) |

## Teknologi

- [axum](https://docs.rs/axum) `0.8` — routing & handler
- [sea-orm](https://www.sea-ql.org/SeaORM/) `2.0` — ORM async + migration
- [jsonwebtoken](https://docs.rs/jsonwebtoken) `11` — JWT (backend `rust_crypto`)
- [argon2](https://docs.rs/argon2) `0.6` — hashing password
- [validator](https://docs.rs/validator) `0.21` — validasi input
- PostgreSQL

## Struktur proyek

```
.
├── migration/                  # crate migrasi SeaORM (tabel users & products)
│   └── src/
│       ├── lib.rs
│       ├── m20240101_000001_create_users.rs
│       └── m20240101_000002_create_products.rs
└── src/
    ├── main.rs                 # entry point: migrasi otomatis + seed admin + server
    ├── config.rs               # baca konfigurasi dari env/.env
    ├── error.rs                # AppError terpusat -> HTTP response
    ├── state.rs                # AppState (db + config) dibagi ke semua handler
    ├── routes.rs               # definisi semua route
    ├── entities/               # entity SeaORM (user, product)
    ├── auth/                   # jwt, password hashing, extractor RBAC
    └── handlers/               # auth, account, profile, product, dashboard
```

## Cara menjalankan

### 1. Siapkan PostgreSQL

Pakai Docker (paling gampang):

```bash
docker compose up -d
```

Atau pakai PostgreSQL yang sudah ada, lalu buat database `axum_playground`.

### 2. Konfigurasi

```bash
cp .env.example .env
```

Sesuaikan `DATABASE_URL` dan `JWT_SECRET` di `.env` bila perlu.

### 3. Jalankan

```bash
cargo run
```

Saat start, aplikasi otomatis:
1. Menjalankan migrasi (membuat tabel `users` & `products`).
2. Membuat **akun admin default** kalau tabel user masih kosong
   (`admin` / `admin123` — bisa diubah via `ADMIN_*` di `.env`).

Server jalan di `http://localhost:3000`.

> Menjalankan migrasi secara manual juga bisa:
> ```bash
> cargo run -p migration -- up      # jalankan migrasi
> cargo run -p migration -- down    # rollback
> cargo run -p migration -- fresh   # drop semua lalu migrasi ulang
> ```

## Daftar Endpoint

| Method | Path | Akses | Keterangan |
|--------|------|-------|------------|
| GET | `/health` | publik | cek status |
| POST | `/auth/register` | publik | daftar user baru (role `staff`) |
| POST | `/auth/login` | publik | login, dapat token JWT |
| GET | `/profile` | login | lihat profil sendiri |
| PUT | `/profile` | login | ubah profil sendiri |
| GET | `/dashboard` | login | ringkasan stok |
| GET | `/products` | login | daftar produk (`?search=`, `?category=`) |
| GET | `/products/{id}` | login | detail produk |
| POST | `/products` | admin | tambah produk |
| PUT | `/products/{id}` | admin | ubah produk |
| PATCH | `/products/{id}/stock` | admin | tambah/kurang stok (`{"delta": -3}`) |
| DELETE | `/products/{id}` | admin | hapus produk |
| GET | `/accounts` | admin | daftar user |
| GET | `/accounts/{id}` | admin | detail user |
| POST | `/accounts` | admin | buat user (bisa set role) |
| PUT | `/accounts/{id}` | admin | ubah nama/role/status user |
| DELETE | `/accounts/{id}` | admin | hapus user |

## Contoh pemakaian (curl)

```bash
# Login admin -> ambil token
TOKEN=$(curl -s localhost:3000/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123"}' | jq -r .token)

# Tambah produk (admin)
curl localhost:3000/products \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"name":"Kopi Arabika","sku":"KOPI-001","category":"minuman","price":85000.50,"quantity":5}'

# Kurangi stok 3
curl -X PATCH localhost:3000/products/1/stock \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"delta":-3}'

# Lihat dashboard
curl localhost:3000/dashboard -H "Authorization: Bearer $TOKEN"

# Register user biasa (role staff)
curl localhost:3000/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"budi","email":"budi@example.com","password":"rahasia123"}'
```

## Catatan untuk belajar

- **RBAC** di sini sengaja simpel: role disimpan sebagai string di kolom `users.role`
  dan dicek lewat extractor `AdminUser`. Untuk kebutuhan nyata, bisa dikembangkan jadi
  tabel `roles` + `permissions` relasional.
- **Password** tidak pernah ikut terkirim ke client (`#[serde(skip_serializing)]` di entity).
- **Dashboard** menghitung agregasi di memori (cukup untuk data kecil). Untuk data besar,
  pakai agregasi SQL (`SUM`/`COUNT`) langsung di query.
- **Error handling** dipusatkan di `AppError` yang mengimplementasikan `IntoResponse`,
  jadi handler cukup mengembalikan `Result<_, AppError>`.
