use std::env;

/// Konfigurasi aplikasi yang dibaca dari environment variable (file `.env`).
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    /// Masa berlaku token dalam jam.
    pub jwt_expires_hours: i64,
    pub server_addr: String,
}

impl Config {
    pub fn from_env() -> Self {
        // Muat .env kalau ada (abaikan error kalau tidak ada file).
        let _ = dotenvy::dotenv();

        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL wajib di-set (lihat .env.example)"),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "ubah-ini-di-produksi".to_string()),
            jwt_expires_hours: env::var("JWT_EXPIRES_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
            server_addr: env::var("SERVER_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
        }
    }
}
