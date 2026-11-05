pub mod account;
pub mod auth;
pub mod dashboard;
pub mod product;
pub mod profile;

use crate::error::AppError;
use validator::Validate;

/// Helper kecil untuk menjalankan validasi `validator` dan mengubah
/// error-nya menjadi `AppError::BadRequest` yang rapi.
pub fn validate<T: Validate>(payload: &T) -> Result<(), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))
}
