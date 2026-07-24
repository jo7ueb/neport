use sqlx::PgPool;

use crate::error::{AppError, Result};

pub async fn upsert(
    db: &PgPool,
    sha256: &str,
    content_type: &str,
    size_bytes: i64,
    uploaded_by_id: Option<i64>,
    access_log_id: Option<i64>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO attachments (sha256, content_type, size_bytes, uploaded_by_id, access_log_id) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (sha256) DO NOTHING",
    )
    .bind(sha256)
    .bind(content_type)
    .bind(size_bytes)
    .bind(uploaded_by_id)
    .bind(access_log_id)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_content_type(db: &PgPool, sha256: &str) -> Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT content_type FROM attachments WHERE sha256 = $1")
        .bind(sha256)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)
}
