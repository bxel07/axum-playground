use axum::{extract::State, Json};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::{create_token, password},
    entities::{prelude::User, user},
    error::{AppError, AppResult},
    handlers::validate,
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50, message = "username 3-50 karakter"))]
    pub username: String,
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(min = 6, message = "password minimal 6 karakter"))]
    pub password: String,
    pub full_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "username wajib diisi"))]
    pub username: String,
    #[validate(length(min = 1, message = "password wajib diisi"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: &'static str,
    pub user: user::Model,
}

/// POST /auth/register — daftar user baru (role default: "staff").
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<user::Model>> {
    validate(&payload)?;

    // Pastikan username / email belum dipakai.
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
        role: Set("staff".to_string()),
        is_active: Set(true),
        ..Default::default()
    };

    let user = new_user.insert(&state.db).await?;
    Ok(Json(user))
}

/// POST /auth/login — tukar username+password menjadi JWT.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    validate(&payload)?;

    let user = User::find()
        .filter(user::Column::Username.eq(&payload.username))
        .one(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !user.is_active {
        return Err(AppError::Forbidden);
    }

    if !password::verify_password(&payload.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let token = create_token(
        user.id,
        &user.username,
        &user.role,
        &state.config.jwt_secret,
        state.config.jwt_expires_hours,
    )
    .map_err(|e| AppError::Internal(format!("gagal membuat token: {e}")))?;

    Ok(Json(LoginResponse {
        token,
        token_type: "Bearer",
        user,
    }))
}
