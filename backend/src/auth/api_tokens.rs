use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiTokenInfo {
    pub id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct CreatedApiToken {
    pub info: ApiTokenInfo,
    pub token: String,
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub async fn create(pool: &PgPool, user_id: i64, name: &str, scopes: &[String]) -> Result<CreatedApiToken> {
    let name = name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest("Token name must contain 1-100 characters".into()));
    }
    let scopes: Vec<String> = scopes.iter().map(|s| s.to_ascii_lowercase()).collect();
    if scopes.is_empty() || scopes.iter().any(|s| s != "read" && s != "write") {
        return Err(AppError::BadRequest("Scopes must contain read and/or write".into()));
    }
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = format!("npt_{}", hex::encode(bytes));
    let id = Uuid::new_v4();
    let info = sqlx::query_as::<_, ApiTokenInfo>(
        "INSERT INTO api_tokens (id, user_id, token_hash, name, scopes) VALUES ($1, $2, $3, $4, $5) RETURNING id, name, scopes, created_at, last_used_at, revoked_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(hash(&token))
    .bind(name)
    .bind(&scopes)
    .fetch_one(pool)
    .await?;
    Ok(CreatedApiToken { info, token })
}

pub async fn list(pool: &PgPool, user_id: i64) -> Result<Vec<ApiTokenInfo>> {
    Ok(sqlx::query_as::<_, ApiTokenInfo>(
        "SELECT id, name, scopes, created_at, last_used_at, revoked_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub async fn revoke(pool: &PgPool, user_id: i64, id: Uuid) -> Result<()> {
    let result = sqlx::query("UPDATE api_tokens SET revoked_at = COALESCE(revoked_at, now()) WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 { return Err(AppError::NotFound); }
    Ok(())
}

pub async fn authenticate(pool: &PgPool, token: &str, write: bool) -> Result<i64> {
    let row = sqlx::query_as::<_, (i64, Vec<String>)>(
        "SELECT user_id, scopes FROM api_tokens WHERE token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(hash(token))
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if write && !row.1.iter().any(|scope| scope == "write") {
        return Err(AppError::Forbidden);
    }
    sqlx::query("UPDATE api_tokens SET last_used_at = now() WHERE token_hash = $1 AND revoked_at IS NULL")
        .bind(hash(token))
        .execute(pool)
        .await?;
    if let Err(error) = crate::audit::repo::record(pool, row.0, "api_token_used", "api_token", None, None, None, None).await {
        tracing::warn!(user_id = row.0, ?error, "failed to record API token usage");
    }
    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use super::hash;

    #[test]
    fn token_hash_is_deterministic_and_not_plaintext() {
        assert_eq!(hash("npt_example"), hash("npt_example"));
        assert_ne!(hash("npt_example"), "npt_example");
        assert_eq!(hash("npt_example").len(), 64);
    }
}
