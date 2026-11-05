# Panduan Integrasi Axum — Case by Case

Dokumen ini menjelaskan, untuk tiap kebutuhan umum di aplikasi production,
**pendekatan mana yang dipakai** (middleware / extractor / handler), **alasannya**,
dan **aturan boleh ✅ / jangan ❌**. Contoh kode mengacu ke tipe yang sudah ada di
repo ini (`AuthUser`, `AdminUser`, `AppError`, `AppState`).

## Peta keputusan cepat

Tanya 3 hal sebelum memilih:

1. **Berlaku untuk banyak route?** → cenderung **middleware (Layer)**.
2. **Butuh konteks per-handler (input, identitas user)?** → **extractor**.
3. **Butuh data bisnis / resource dari DB untuk memutuskan?** → **di handler/service**.

| Kebutuhan | Pendekatan | Alasan singkat |
|-----------|-----------|----------------|
| Logging/tracing | Middleware `TraceLayer` | lintas-route, tak ada logika bisnis |
| CORS | Middleware `CorsLayer` | lintas-route |
| Timeout | Middleware `TimeoutLayer` | lintas-route |
| Batas ukuran body | Middleware `RequestBodyLimitLayer` | lintas-route, proteksi |
| Kompresi response | Middleware `CompressionLayer` | lintas-route |
| Rate limiting | Middleware (`tower_governor`) | lintas-route, proteksi |
| Verifikasi token (authN) | Middleware **atau** extractor (hybrid terbaik) | identitas, bisa per-grup |
| Role check (authZ sederhana) | Extractor (`AdminUser`) | per-handler, dari claims |
| Ownership / permission granular | **Di handler/service** | butuh data resource dari DB |
| Parsing & validasi input | Extractor (`Json<T>` + `validator`) | per-handler |
| Akses DB & config | Extractor `State<AppState>` | type-safe, bukan middleware |
| Request ID / correlation | Middleware set → extractor baca | lintas-route + dipakai handler |
| Format error | `impl IntoResponse` (`AppError`) | bukan tugas middleware |

---

## Case 1 — Logging / Tracing

**Pakai: middleware `TraceLayer`.**

Observability berlaku ke semua request dan tidak bergantung logika bisnis, jadi ini
contoh klasik middleware.

```rust
use tower_http::trace::TraceLayer;

app.layer(TraceLayer::new_for_http())
```

- ✅ Pasang paling luar supaya mencatat seluruh durasi request (termасuk middleware lain).
- ✅ Gabungkan dengan `tracing_subscriber` (sudah ada di `main.rs`).
- ❌ Jangan log body request yang berisi password/token mentah.
- ❌ Jangan bikin logging manual di tiap handler kalau TraceLayer sudah menangani.

---

## Case 2 — CORS

**Pakai: middleware `CorsLayer`.** (Sudah ada di `routes.rs`.)

```rust
use tower_http::cors::CorsLayer;
use axum::http::{HeaderValue, Method};

// Production: JANGAN permissive. Batasi origin eksplisit.
CorsLayer::new()
    .allow_origin("https://app.domain.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST])
    .allow_credentials(true)
```

- ✅ Di production, whitelist origin yang spesifik.
- ❌ **Jangan** `CorsLayer::permissive()` di production (kita pakai ini di playground
  hanya untuk kemudahan belajar).
- ❌ Jangan kombinasikan `allow_origin(Any)` dengan `allow_credentials(true)` — ditolak browser.

---

## Case 3 — Timeout & Case 4 — Batas ukuran body

**Pakai: middleware.** Keduanya proteksi lintas-route.

```rust
use std::time::Duration;
use tower_http::{timeout::TimeoutLayer, limit::RequestBodyLimitLayer};

app.layer(TimeoutLayer::new(Duration::from_secs(10)))
   .layer(RequestBodyLimitLayer::new(1024 * 1024)) // maks 1 MB
```

- ✅ Selalu pasang body limit di API publik (cegah payload besar / DoS).
- ✅ Sesuaikan timeout dengan operasi terlama yang wajar.
- ❌ Jangan set timeout lebih pendek dari operasi normal (mis. upload) → request valid ikut gagal.

---

## Case 5 — Verifikasi token (Authentication)

**Rekomendasi production: HYBRID — middleware verifikasi sekali → simpan ke
`Extensions` → extractor baca.** Di playground ini kita pakai extractor-only (sah juga,
tapi token di-decode tiap kali extractor dipanggil).

```rust
use axum::{middleware::{self, Next}, extract::{Request, State}};

async fn auth_mw(State(st): State<AppState>, mut req: Request, next: Next)
    -> Result<axum::response::Response, AppError>
{
    let claims = verifikasi_token_dari_header(&req, &st)?; // 401 bila gagal
    req.extensions_mut().insert(claims);                   // decode SEKALI
    Ok(next.run(req).await)
}

// Extractor jadi murah: cukup baca extensions.
```

Pasang ke **grup** dengan `route_layer`, bukan global:

```rust
let protected = Router::new()
    .route("/products", post(create_product))
    .route_layer(middleware::from_fn_with_state(state.clone(), auth_mw));

let app = Router::new()
    .route("/auth/login", post(login))   // publik
    .merge(protected);
```

- ✅ Decode token **sekali per request** (hybrid), bukan berulang.
- ✅ Pakai `route_layer` → path tak dikenal tetap `404`, bukan `401`.
- ✅ Kelompokkan route publik vs terproteksi secara eksplisit.
- ❌ Jangan `.layer(auth)` global kalau ada route publik (`/login`, `/health`) → semua kena 401.
- ❌ Jangan taruh token/claims di state global yang dibagi antar request — claims milik **satu** request, taruh di `Extensions`.

---

## Case 6 — Role check (Authorization sederhana)

**Pakai: extractor (`AdminUser`).** Keputusannya hanya butuh `role` dari claims, per-handler.
(Sudah ada di `src/auth/mod.rs`.)

```rust
async fn create_product(_admin: AdminUser, ...) { /* hanya admin */ }
```

- ✅ Pakai tipe extractor berbeda untuk level akses berbeda (`AuthUser` vs `AdminUser`).
- ✅ Cek role dari claims JWT (tidak perlu query DB untuk role sederhana).
- ❌ Jangan hardcode cek `if role == "admin"` berulang di dalam handler — bungkus jadi extractor.
- ❌ Jangan pakai middleware untuk role check yang beda-beda tiap endpoint → lebih pas extractor.

---

## Case 7 — Ownership / permission granular (Authorization kompleks)

**Pakai: di dalam handler/service.** Keputusan butuh **data resource dari DB**
(mis. "user hanya boleh mengedit produk miliknya"), sesuatu yang middleware/extractor
tidak punya konteksnya.

```rust
async fn update_product(State(st): State<AppState>, auth: AuthUser, Path(id): Path<i32>, ...)
    -> AppResult<Json<product::Model>>
{
    let p = Product::find_by_id(id).one(&st.db).await?
        .ok_or_else(|| AppError::NotFound("produk".into()))?;

    // authorization berbasis data: perlu tahu pemilik resource
    if !auth.is_admin() && p.created_by != Some(auth.user_id) {
        return Err(AppError::Forbidden);
    }
    // ... lanjut update
}
```

- ✅ Lakukan cek ownership setelah memuat resource, sebelum memutasi.
- ✅ Untuk aturan yang banyak & dinamis, pertimbangkan library policy (mis. `casbin`).
- ❌ Jangan paksakan logika ini ke middleware/extractor — mereka tidak tahu resource mana yang diakses.
- ❌ Jangan andalkan "sembunyikan tombol di frontend" sebagai authorization. Backend tetap wajib cek.

---

## Case 8 — Parsing & validasi input

**Pakai: extractor `Json<T>` + `validator`.** (Sudah ada pola `validate()` di `handlers/mod.rs`.)

```rust
#[derive(Deserialize, Validate)]
struct CreateProductRequest {
    #[validate(length(min = 1))] name: String,
    #[validate(range(min = 0))] quantity: i32,
}

async fn create_product(Json(payload): Json<CreateProductRequest>) -> AppResult<_> {
    validate(&payload)?; // -> 400 bila gagal
}
```

- ✅ Validasi sedekat mungkin dengan titik masuk (extractor/awal handler).
- ✅ Urutan extractor penting: yang **mengonsumsi body** (`Json`) harus **argumen terakhir**.
- ❌ Jangan percaya input client; selalu validasi (panjang, range, enum seperti role).
- ❌ Jangan taruh `Json<T>` sebelum extractor lain di signature handler → error kompilasi/runtime.

---

## Case 9 — Akses DB & konfigurasi (shared state)

**Pakai: extractor `State<AppState>`, BUKAN middleware/Extension.** (Sudah dipakai.)

```rust
async fn handler(State(st): State<AppState>) { st.db; st.config; }
```

- ✅ Pakai `State` untuk dependency yang dibagi (pool DB, config) — type-safe, dicek compiler.
- ✅ `AppState` cukup `#[derive(Clone)]`; `DatabaseConnection` murah di-clone (Arc internal).
- ❌ Jangan pakai `Extension<T>` untuk ini kalau `State` bisa — `Extension` baru gagal saat runtime bila lupa dipasang, `State` gagal saat compile.
- ❌ Jangan bikin koneksi DB baru per request — pakai satu pool di `AppState`.

---

## Case 10 — Request ID / correlation ID

**Pakai: middleware (set) → extractor (baca) via `Extensions`.** Pola "middleware menitipkan
data untuk handler".

```rust
// middleware: generate id, titip ke extensions
req.extensions_mut().insert(RequestId(uuid));
// extractor/handler: baca dari parts.extensions
```

- ✅ Inilah cara sah middleware "mengirim" data ke handler: lewat `Extensions`.
- ❌ Jangan pakai `Extensions` untuk data yang tipenya tetap & selalu ada → pakai `State`.

---

## Case 11 — Format & handling error

**Pakai: `impl IntoResponse` pada tipe error (`AppError`), BUKAN middleware.** (Sudah ada di `error.rs`.)

```rust
async fn handler() -> Result<Json<T>, AppError> { ... } // error otomatis jadi HTTP response
```

- ✅ Satu tipe error terpusat yang tahu cara jadi response (status + body JSON).
- ✅ Pakai `?` untuk propagasi; map error eksternal ke varian `AppError`.
- ❌ Jangan `.unwrap()` / `.expect()` di handler (panic = 500 tanpa kendali, bisa menjatuhkan task).
- ❌ Jangan bocorkan detail internal (SQL error mentah) ke client — log detail, balas pesan umum
  (lihat varian `AppError::Database`).

---

## Aturan umum (boleh ✅ / jangan ❌)

**Urutan & struktur**
- ✅ Atur tumpukan layer pakai `tower::ServiceBuilder` agar urutan eksplisit (atas = terluar).
- ✅ Letakkan `TraceLayer` terluar, proteksi (timeout/body-limit) di dalamnya.
- ❌ Jangan asal `.layer()` berurutan tanpa paham arah pembungkusan (bawah→atas).

**Pemisahan tanggung jawab**
- ✅ Middleware = urusan infrastruktur (log, cors, timeout, limit, rate-limit).
- ✅ Extractor = identitas + input per-handler.
- ✅ Handler/service = logika bisnis + authorization berbasis data.
- ❌ Jangan taruh logika bisnis di middleware.
- ❌ Jangan ulang-ulang kerjaan yang sama di tiap handler kalau bisa jadi middleware/extractor.

**Performa & keamanan**
- ✅ Bungkus kerja CPU berat (hashing Argon2) dengan `tokio::task::spawn_blocking` di
  production agar tidak memblokir runtime async.
- ✅ Selalu: body limit + timeout + CORS ketat + validasi input di API publik.
- ❌ Jangan decode JWT berkali-kali per request — verifikasi sekali, simpan ke `Extensions`.
- ❌ Jangan `permissive()` apa pun (CORS, dll.) di production.

**State vs Extension**
- ✅ Dependency tetap (DB, config) → `State` (type-safe).
- ✅ Data per-request dari middleware (claims, request id) → `Extensions`.
- ❌ Jangan tukar keduanya.

---

## Ringkasan satu kalimat

> **Middleware** untuk hal yang sama di banyak route (infra & proteksi),
> **extractor** untuk identitas & input tiap handler,
> **handler/service** untuk keputusan yang butuh data bisnis.
> Ketiganya saling melengkapi, bukan saling menggantikan.
