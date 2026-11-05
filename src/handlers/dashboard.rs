use axum::{extract::State, Json};
use rust_decimal::Decimal;
use sea_orm::EntityTrait;
use serde::Serialize;

use crate::{
    auth::AuthUser,
    entities::{prelude::Product, product},
    error::AppResult,
    state::AppState,
};

/// Ambang batas stok rendah untuk ditandai di dashboard.
const LOW_STOCK_THRESHOLD: i32 = 10;

#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    pub total_products: usize,
    pub total_quantity: i64,
    pub total_stock_value: Decimal,
    pub low_stock_count: usize,
    pub out_of_stock_count: usize,
    pub low_stock_items: Vec<product::Model>,
}

/// GET /dashboard — ringkasan stok sederhana.
///
/// Dihitung di memori (cukup untuk playground). Untuk data besar sebaiknya
/// memakai agregasi SQL (SUM/COUNT) langsung di database.
pub async fn summary(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> AppResult<Json<DashboardSummary>> {
    let products = Product::find().all(&state.db).await?;

    let total_products = products.len();
    let mut total_quantity: i64 = 0;
    let mut total_stock_value = Decimal::ZERO;
    let mut out_of_stock_count = 0usize;
    let mut low_stock_items = Vec::new();

    for p in &products {
        total_quantity += p.quantity as i64;
        total_stock_value += p.price * Decimal::from(p.quantity);

        if p.quantity == 0 {
            out_of_stock_count += 1;
        }
        if p.quantity < LOW_STOCK_THRESHOLD {
            low_stock_items.push(p.clone());
        }
    }

    Ok(Json(DashboardSummary {
        total_products,
        total_quantity,
        total_stock_value,
        low_stock_count: low_stock_items.len(),
        out_of_stock_count,
        low_stock_items,
    }))
}
