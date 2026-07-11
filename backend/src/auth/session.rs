use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::Result;

use super::csrf::generate_csrf_token;

pub struct Session {
    pub id: Uuid,
    pub user_id: i64,
    pub csrf_token: String,
}

pub async fn create_session(pool: &PgPool, user_id: i64, ttl_seconds: u64) -> Result<Session> {
    let session_id = Uuid::new_v4();
    let csrf_token = generate_csrf_token();
    let expires_at = Utc::now() + Duration::seconds(ttl_seconds as i64);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(&csrf_token)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(Session {
        id: session_id,
        user_id,
        csrf_token,
    })
}

pub async fn get_session(pool: &PgPool, session_id: Uuid) -> Result<Option<Session>> {
    let row = sqlx::query(
        "SELECT id, user_id, csrf_token FROM sessions WHERE id = $1 AND expires_at > NOW()",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Session {
        id: r.get(0),
        user_id: r.get(1),
        csrf_token: r.get(2),
    }))
}

pub async fn delete_session(pool: &PgPool, session_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Invalidates every session for a user except `keep_session_id`. Used after
/// a self-service password change so a leaked/stolen session elsewhere is
/// killed immediately, without logging the user out of the session they just
/// used to make the change.
pub async fn delete_other_sessions_for_user(
    pool: &PgPool,
    user_id: i64,
    keep_session_id: Uuid,
) -> Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND id != $2")
        .bind(user_id)
        .bind(keep_session_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Invalidates every session for a user, including any currently active one.
/// Used after an admin-triggered password reset, since the admin (not the
/// user) initiated it and there's no "session to keep".
pub async fn delete_all_sessions_for_user(pool: &PgPool, user_id: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
