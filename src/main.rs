mod auth;
mod config;
mod entities;
mod error;
mod handlers;
mod routes;
mod state;

use config::Config;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ActiveModelTrait, ColumnTrait, Database, EntityTrait, QueryFilter, Set};
use state::AppState;
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    auth::password,
    entities::{prelude::User, user},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Logging ---
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum_playground=debug,tower_http=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // --- Konfigurasi & koneksi DB ---
    let config = Config::from_env();
    let db = Database::connect(&config.database_url).await?;

    // Jalankan migrasi otomatis saat startup (praktis untuk belajar).
    Migrator::up(&db, None).await?;

    // Seed akun admin pertama kalau tabel users masih kosong.
    seed_admin(&db).await?;

    let state = AppState {
        db,
        config: config.clone(),
    };

    // --- Server ---
    let app = routes::build_router(state);
    let listener = TcpListener::bind(&config.server_addr).await?;
    tracing::info!("🚀 server jalan di http://{}", config.server_addr);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Buat admin default dari env (ADMIN_USERNAME / ADMIN_EMAIL / ADMIN_PASSWORD)
/// hanya ketika belum ada user sama sekali.
async fn seed_admin(db: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
    let has_user = User::find().one(db).await?.is_some();
    if has_user {
        return Ok(());
    }

    let username = env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let email = env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".to_string());
    let plain = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

    // Hindari duplikat kalau ternyata username/email sudah ada (jaga-jaga).
    let existing = User::find()
        .filter(user::Column::Username.eq(&username))
        .one(db)
        .await?;
    if existing.is_some() {
        return Ok(());
    }

    let hash = password::hash_password(&plain)
        .map_err(|e| anyhow::anyhow!("gagal hash password admin: {e}"))?;

    let admin = user::ActiveModel {
        username: Set(username.clone()),
        email: Set(email),
        password_hash: Set(hash),
        full_name: Set(Some("Administrator".to_string())),
        role: Set("admin".to_string()),
        is_active: Set(true),
        ..Default::default()
    };
    admin.insert(db).await?;

    tracing::info!("👤 akun admin default dibuat: username='{username}' (ganti password-nya!)");
    Ok(())
}
