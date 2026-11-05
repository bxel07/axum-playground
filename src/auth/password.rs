use argon2::{password_hash::Error, Argon2, PasswordHasher, PasswordVerifier};

/// Hash password memakai Argon2. Salt acak dibuat otomatis oleh Argon2.
pub fn hash_password(password: &str) -> Result<String, Error> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string();
    Ok(hash)
}

/// Cek apakah password cocok dengan hash (format PHC) yang tersimpan.
pub fn verify_password(password: &str, hash: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), hash)
        .is_ok()
}
