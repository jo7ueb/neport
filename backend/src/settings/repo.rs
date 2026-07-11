use sqlx::PgPool;

use crate::error::{AppError, Result};
use super::models::SystemSettings;

pub async fn get_settings(db: &PgPool) -> Result<SystemSettings> {
    sqlx::query_as::<_, (bool, String, bool)>(
        "SELECT katex_enabled, default_timezone, maintenance_mode FROM system_settings WHERE id = 1"
    )
        .fetch_one(db)
        .await
        .map_err(AppError::Database)
        .map(|(katex_enabled, default_timezone, maintenance_mode)| SystemSettings { katex_enabled, default_timezone, maintenance_mode })
}

pub async fn update_settings(
    db: &PgPool,
    katex_enabled: bool,
    default_timezone: &str,
    maintenance_mode: bool,
) -> Result<SystemSettings> {
    sqlx::query_as::<_, (bool, String, bool)>(
        "UPDATE system_settings SET katex_enabled = $1, default_timezone = $2, maintenance_mode = $3, updated_at = NOW() WHERE id = 1 \
         RETURNING katex_enabled, default_timezone, maintenance_mode"
    )
    .bind(katex_enabled)
    .bind(default_timezone)
    .bind(maintenance_mode)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(|(katex_enabled, default_timezone, maintenance_mode)| SystemSettings { katex_enabled, default_timezone, maintenance_mode })
}

// Cheap standalone check for auth::extractors::AuthUser to call on every
// non-GET request - a single-column, single-row lookup against a table this
// small stays fast, and avoids threading the full SystemSettings type
// through the auth layer just for one bool.
pub async fn is_maintenance_mode(db: &PgPool) -> Result<bool> {
    let (maintenance_mode,): (bool,) = sqlx::query_as("SELECT maintenance_mode FROM system_settings WHERE id = 1")
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(maintenance_mode)
}
