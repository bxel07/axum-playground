use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    auth::{AdminUser, AuthUser},
    entities::{prelude::Product, product},
    error::{AppError, AppResult},
    handlers::validate,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Filter berdasarkan nama/SKU (case-insensitive).
    pub search: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, message = "nama wajib diisi"))]
    pub name: String,
    #[validate(length(min = 1, message = "sku wajib diisi"))]
    pub sku: String,
    pub category: Option<String>,
    pub price: Decimal,
    #[validate(range(min = 0, message = "quantity tidak boleh negatif"))]
    pub quantity: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub price: Option<Decimal>,
    #[validate(range(min = 0, message = "quantity tidak boleh negatif"))]
    pub quantity: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AdjustStockRequest {
    /// Positif untuk menambah stok, negatif untuk mengurangi.
    pub delta: i32,
}

/// GET /products — daftar produk/stok. Bisa diakses semua user login.
pub async fn list_products(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<product::Model>>> {
    let mut query = Product::find();

    if let Some(search) = q.search.filter(|s| !s.is_empty()) {
        let pattern = format!("%{search}%");
        query = query.filter(
            sea_orm::Condition::any()
                .add(product::Column::Name.like(&pattern))
                .add(product::Column::Sku.like(&pattern)),
        );
    }
    if let Some(category) = q.category.filter(|s| !s.is_empty()) {
        query = query.filter(product::Column::Category.eq(category));
    }

    let products = query
        .order_by_asc(product::Column::Id)
        .all(&state.db)
        .await?;
    Ok(Json(products))
}

/// GET /products/{id} — detail produk.
pub async fn get_product(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<product::Model>> {
    let product = Product::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("produk".to_string()))?;
    Ok(Json(product))
}

/// POST /products — tambah produk baru (admin).
pub async fn create_product(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<CreateProductRequest>,
) -> AppResult<Json<product::Model>> {
    validate(&payload)?;

    let exists = Product::find()
        .filter(product::Column::Sku.eq(&payload.sku))
        .one(&state.db)
        .await?;
    if exists.is_some() {
        return Err(AppError::Conflict("SKU sudah dipakai".to_string()));
    }

    let now = chrono::Utc::now().fixed_offset();
    let new_product = product::ActiveModel {
        name: Set(payload.name),
        sku: Set(payload.sku),
        category: Set(payload.category.unwrap_or_else(|| "umum".to_string())),
        price: Set(payload.price),
        quantity: Set(payload.quantity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let product = new_product.insert(&state.db).await?;
    Ok(Json(product))
}

/// PUT /products/{id} — ubah data produk (admin).
pub async fn update_product(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProductRequest>,
) -> AppResult<Json<product::Model>> {
    validate(&payload)?;

    let product = Product::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("produk".to_string()))?;

    let mut active: product::ActiveModel = product.into();

    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(category) = payload.category {
        active.category = Set(category);
    }
    if let Some(price) = payload.price {
        active.price = Set(price);
    }
    if let Some(quantity) = payload.quantity {
        active.quantity = Set(quantity);
    }
    active.updated_at = Set(chrono::Utc::now().fixed_offset());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

/// PATCH /products/{id}/stock — tambah/kurangi stok (admin).
pub async fn adjust_stock(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
    Json(payload): Json<AdjustStockRequest>,
) -> AppResult<Json<product::Model>> {
    let product = Product::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("produk".to_string()))?;

    let new_qty = product.quantity + payload.delta;
    if new_qty < 0 {
        return Err(AppError::BadRequest(format!(
            "stok tidak cukup (sekarang {}, diminta {})",
            product.quantity, payload.delta
        )));
    }

    let mut active: product::ActiveModel = product.into();
    active.quantity = Set(new_qty);
    active.updated_at = Set(chrono::Utc::now().fixed_offset());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

/// DELETE /products/{id} — hapus produk (admin).
pub async fn delete_product(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
) -> AppResult<Json<serde_json::Value>> {
    let res = Product::delete_by_id(id).exec(&state.db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound("produk".to_string()));
    }
    Ok(Json(serde_json::json!({ "deleted": id })))
}
