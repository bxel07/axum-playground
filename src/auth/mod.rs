pub mod jwt;
pub mod password;

pub use jwt::create_token;

use crate::{error::AppError, state::AppState};
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

/// Extractor yang mewakili user yang sudah login.
///
/// Taruh ini sebagai argumen handler, mis. `async fn me(user: AuthUser)` dan
/// Axum otomatis memvalidasi JWT di header `Authorization: Bearer <token>`.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = jwt::verify_token(token, &state.config.jwt_secret)
            .map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser {
            user_id: claims.sub,
            role: claims.role,
        })
    }
}

/// Versi `AuthUser` yang hanya lolos kalau role-nya "admin".
///
/// Inilah bentuk RBAC sederhana: cukup minta `AdminUser` di handler yang
/// butuh hak admin, user biasa langsung ditolak dengan 403.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.is_admin() {
            Ok(AdminUser(user))
        } else {
            Err(AppError::Forbidden)
        }
    }
}
