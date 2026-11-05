use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Error terpusat untuk seluruh aplikasi.
///
/// Setiap variant tahu caranya berubah menjadi HTTP response lewat
/// implementasi `IntoResponse`, jadi handler cukup mengembalikan `Result<_, AppError>`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("tidak terautentikasi")]
    Unauthorized,

    #[error("tidak punya akses")]
    Forbidden,

    #[error("{0} tidak ditemukan")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("error database: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("error internal: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::Database(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "terjadi kesalahan pada database".to_string(),
                )
            }
            AppError::Internal(e) => {
                tracing::error!("internal error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "terjadi kesalahan internal".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
