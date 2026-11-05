use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    auth::{password, AdminUser},
    entities::{prelude::User, user},
    error::{AppError, AppResult},
    handlers::validate,
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50, message = "username 3-50 karakter"))]
    pub username: String,
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(min = 6, message = "password minimal 6 karakter"))]
    pub password: String,
    pub full_name: Option<String>,
    /// "admin" atau "staff". Default "staff" bila kosong.
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub full_name: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

fn normalize_role(role: &str) -> AppResult<String> {
    match role {
        "admin" | "staff" => Ok(role.to_string()),
        _ => Err(AppError::BadRequest(
            "role harus 'admin' atau 'staff'".to_string(),
        )),
    }
}

/// GET /accounts — daftar semua user (admin).
pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<user::Model>>> {
    let users = User::find()
        .order_by_asc(user::Column::Id)
        .all(&state.db)
        .await?;
    Ok(Json(users))
}

/// GET /accounts/{id} — detail satu user (admin).
pub async fn get_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
) -> AppResult<Json<user::Model>> {
    let user = User::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("user".to_string()))?;
    Ok(Json(user))
}

/// POST /accounts — buat user baru dengan role tertentu (admin).
pub async fn create_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<CreateUserRequest>,
) -> AppResult<Json<user::Model>> {
    validate(&payload)?;

    let role = normalize_role(payload.role.as_deref().unwrap_or("staff"))?;

    let existing = User::find()
        .filter(
            Condition::any()
                .add(user::Column::Username.eq(&payload.username))
                .add(user::Column::Email.eq(&payload.email)),
        )
        .one(&state.db)
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "username atau email sudah terdaftar".to_string(),
        ));
    }

    let password_hash = password::hash_password(&payload.password)
        .map_err(|e| AppError::Internal(format!("gagal hash password: {e}")))?;

    let new_user = user::ActiveModel {
        username: Set(payload.username),
        email: Set(payload.email),
        password_hash: Set(password_hash),
        full_name: Set(payload.full_name),
        role: Set(role),
        is_active: Set(true),
        ..Default::default()
    };

    let user = new_user.insert(&state.db).await?;
    Ok(Json(user))
}

/// PUT /accounts/{id} — ubah role / status / nama user (admin).
pub async fn update_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<user::Model>> {
    let user = User::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("user".to_string()))?;

    let mut active: user::ActiveModel = user.into();

    if let Some(full_name) = payload.full_name {
        active.full_name = Set(Some(full_name));
    }
    if let Some(role) = payload.role {
        active.role = Set(normalize_role(&role)?);
    }
    if let Some(is_active) = payload.is_active {
        active.is_active = Set(is_active);
    }

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

/// DELETE /accounts/{id} — hapus user (admin). Tidak boleh menghapus diri sendiri.
pub async fn delete_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<i32>,
) -> AppResult<Json<serde_json::Value>> {
    if admin.0.user_id == id {
        return Err(AppError::BadRequest(
            "tidak bisa menghapus akun sendiri".to_string(),
        ));
    }

    let res = User::delete_by_id(id).exec(&state.db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound("user".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": id })))
}
