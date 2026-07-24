use sqlx::PgPool;

use crate::error::{AppError, Result};

// Central "who did this, from where, when" record. Every table that tracks
// its own mutating events (audit_log itself, plus the several domain tables
// audit::repo::ENTRIES_SQL merges into the admin audit log - see that file's
// module comment) stores a FK to a row created here instead of its own
// separate ip_address column, so IP capture only has to be implemented once.
pub async fn record(db: &PgPool, actor_id: i64, ip_address: Option<&str>) -> Result<i64> {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO access_log (actor_id, ip_address, occurred_at) VALUES ($1, $2, NOW()) RETURNING id",
    )
    .bind(actor_id)
    .bind(ip_address)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;
    Ok(id)
}
