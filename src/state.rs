use crate::config::Config;
use sea_orm::DatabaseConnection;

/// State yang dibagikan ke semua handler lewat `State` extractor Axum.
/// `Clone` murah karena `DatabaseConnection` dan `Config` memakai `Arc` di dalamnya.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
}
