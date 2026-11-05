use axum::{extract::State, Json};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use validator::Validate;

use crate::{
    auth::{password, AuthUser},
    entities::{prelude::User, user},
    error::{AppError, AppResult},
    handlers::validate,
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,
    #[validate(email(message = "email tidak valid"))]
    pub email: Option<String>,
    #[validate(length(min = 6, message = "password minimal 6 karakter"))]
    pub password: Option<String>,
}

/// GET /profile — lihat data diri sendiri.
pub async fn get_profile(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<user::Model>> {
    let user = User::find_by_id(auth.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("user".to_string()))?;
    Ok(Json(user))
}

/// PUT /profile — ubah data diri sendiri (nama, email, atau password).
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> AppResult<Json<user::Model>> {
    validate(&payload)?;

    let user = User::find_by_id(auth.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("user".to_string()))?;

    let mut active: user::ActiveModel = user.into();

    if let Some(full_name) = payload.full_name {
        active.full_name = Set(Some(full_name));
    }
    if let Some(email) = payload.email {
        active.email = Set(email);
    }
    if let Some(pw) = payload.password {
        let hash = password::hash_password(&pw)
            .map_err(|e| AppError::Internal(format!("gagal hash password: {e}")))?;
        active.password_hash = Set(hash);
    }

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}
