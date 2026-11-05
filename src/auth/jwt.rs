use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Isi (payload) dari JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject = id user.
    pub sub: i32,
    pub username: String,
    pub role: String,
    /// Waktu kedaluwarsa (unix timestamp, detik).
    pub exp: i64,
}

/// Buat token baru untuk user yang berhasil login.
pub fn create_token(
    user_id: i32,
    username: &str,
    role: &str,
    secret: &str,
    expires_hours: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(expires_hours)).timestamp();

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role: role.to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validasi token dan kembalikan claims-nya. Otomatis mengecek `exp`.
pub fn verify_token(
    token: &str,
    secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
