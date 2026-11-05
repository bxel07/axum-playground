use axum::{
    routing::{get, patch, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;

use crate::{
    handlers::{account, auth, dashboard, product, profile},
    state::AppState,
};

/// Rakit seluruh rute aplikasi menjadi satu `Router`.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        // --- Auth (publik) ---
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        // --- Profile (user login) ---
        .route("/profile", get(profile::get_profile).put(profile::update_profile))
        // --- Account management (admin) ---
        .route("/accounts", get(account::list_users).post(account::create_user))
        .route(
            "/accounts/{id}",
            get(account::get_user)
                .put(account::update_user)
                .delete(account::delete_user),
        )
        // --- Dashboard stok (user login) ---
        .route("/dashboard", get(dashboard::summary))
        // --- Menu / produk stok ---
        .route(
            "/products",
            get(product::list_products).post(product::create_product),
        )
        .route(
            "/products/{id}",
            get(product::get_product)
                .put(product::update_product)
                .delete(product::delete_product),
        )
        .route("/products/{id}/stock", patch(product::adjust_stock))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
