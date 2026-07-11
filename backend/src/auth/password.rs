use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("Hash error")]
    Hash,
}

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(rand::thread_rng());
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| PasswordError::Hash)?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| PasswordError::Hash)?;
    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Minimum strength for user-supplied passwords (account creation, self-service
/// change). Admin-generated temporary passwords bypass this since
/// generate_temp_password() already guarantees a stronger random result.
pub fn validate_strength(password: &str) -> Result<(), crate::error::AppError> {
    if password.chars().count() < 8 {
        return Err(crate::error::AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err(crate::error::AppError::BadRequest(
            "password must contain both letters and numbers".into(),
        ));
    }
    Ok(())
}

/// A random temporary password for admin-triggered resets. Excludes visually
/// confusable characters (0/O, 1/I/l) since it must be read aloud or typed
/// from a chat message by the admin communicating it to the user.
pub fn generate_temp_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "my_secure_password_123";
        let hash = hash_password(password).unwrap();
        assert_ne!(hash, password);
        assert!(verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_verify_wrong_password() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_validate_strength_rejects_short_passwords() {
        assert!(validate_strength("Ab1").is_err());
    }

    #[test]
    fn test_validate_strength_rejects_letters_only() {
        assert!(validate_strength("onlyletters").is_err());
    }

    #[test]
    fn test_validate_strength_rejects_digits_only() {
        assert!(validate_strength("12345678").is_err());
    }

    #[test]
    fn test_validate_strength_accepts_letters_and_digits() {
        assert!(validate_strength("GoodPass123").is_ok());
    }
}
