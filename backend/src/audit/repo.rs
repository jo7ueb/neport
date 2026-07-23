use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::error::{AppError, Result};
use super::models::AuditLogEntry;

type AuditLogRow = (
    i64,
    i64,
    String,
    String,
    String,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

// target_id is polymorphic across target_type (user/project/ticket/...), so
// it can't carry a real FK - resolve a human-readable label per known
// target_type instead. Workflow status/transition deletions carry their
// label in from_text directly (no lookup table lends itself to a JOIN there
// since the row is already gone by the time it's recorded), and
// system_settings has no target_id at all.
const SELECT_SQL: &str = "SELECT a.id, a.actor_id, actor.display_name, a.action, a.target_type, a.target_id,
        COALESCE(tu.display_name, tp.name, tt.title, tw.name), a.from_text, a.to_text, a.ip_address, a.occurred_at
     FROM audit_log a
     JOIN users actor ON actor.id = a.actor_id
     LEFT JOIN users tu ON tu.id = a.target_id AND a.target_type IN ('user', 'user_approver', 'project_member')
     LEFT JOIN projects tp ON tp.id = a.target_id AND a.target_type = 'project'
     LEFT JOIN tickets tt ON tt.id = a.target_id AND a.target_type = 'ticket'
     LEFT JOIN workflows tw ON tw.id = a.target_id AND a.target_type = 'workflow'";

fn row_to_entry(row: AuditLogRow) -> AuditLogEntry {
    let (id, actor_id, actor_name, action, target_type, target_id, target_name, from_text, to_text, ip_address, occurred_at) = row;
    AuditLogEntry { id, actor_id, actor_name, action, target_type, target_id, target_name, from_text, to_text, ip_address, occurred_at }
}

// from/to are plain calendar dates (as offered by an <input type="date">
// picker), not timestamps - there's no single meaningful timezone for a
// system-wide log spanning every user, so the range is treated as whole UTC
// days: [from 00:00, to+1day 00:00).
pub async fn list(
    db: &PgPool,
    actor_id: Option<i64>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLogEntry>> {
    let sql = format!(
        "{SELECT_SQL} WHERE ($1::bigint IS NULL OR a.actor_id = $1)
           AND ($2::date IS NULL OR a.occurred_at >= $2::date)
           AND ($3::date IS NULL OR a.occurred_at < $3::date + INTERVAL '1 day')
         ORDER BY a.occurred_at DESC LIMIT $4 OFFSET $5"
    );
    sqlx::query_as::<_, AuditLogRow>(&sql)
        .bind(actor_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_entry).collect())
}

pub async fn count(db: &PgPool, actor_id: Option<i64>, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_log a
         WHERE ($1::bigint IS NULL OR a.actor_id = $1)
           AND ($2::date IS NULL OR a.occurred_at >= $2::date)
           AND ($3::date IS NULL OR a.occurred_at < $3::date + INTERVAL '1 day')",
    )
    .bind(actor_id)
    .bind(from)
    .bind(to)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;
    Ok(count)
}

// System-wide audit trail for actions that mutate or remove an existing row
// (suspending/deleting a user, archiving a project, resetting a password,
// etc.) - see ticket_activity_log for the ticket-scoped equivalent.
pub async fn record(
    db: &PgPool,
    actor_id: i64,
    action: &str,
    target_type: &str,
    target_id: Option<i64>,
    from_text: Option<&str>,
    to_text: Option<&str>,
    ip_address: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO audit_log (actor_id, action, target_type, target_id, from_text, to_text, ip_address, occurred_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(from_text)
    .bind(to_text)
    .bind(ip_address)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}
