use chrono::NaiveDate;
use sqlx::PgPool;

use crate::error::{AppError, Result};
use super::models::{ActivityType, TicketActivity};

const ACTIVITY_COLUMNS: &str = "id, ticket_id, user_id, activity_type, occurred_at, \
    from_status_id, to_status_id, from_assignee_id, to_assignee_id, field_name, from_text, to_text";

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: i64,
    ticket_id: i64,
    user_id: i64,
    activity_type: String,
    occurred_at: chrono::DateTime<chrono::Utc>,
    from_status_id: Option<i64>,
    to_status_id: Option<i64>,
    from_assignee_id: Option<i64>,
    to_assignee_id: Option<i64>,
    field_name: Option<String>,
    from_text: Option<String>,
    to_text: Option<String>,
}

fn row_to_activity(row: ActivityRow) -> TicketActivity {
    TicketActivity {
        id: row.id,
        ticket_id: row.ticket_id,
        user_id: row.user_id,
        activity_type: row.activity_type.parse().unwrap_or(ActivityType::FieldUpdate),
        occurred_at: row.occurred_at,
        from_status_id: row.from_status_id,
        to_status_id: row.to_status_id,
        from_assignee_id: row.from_assignee_id,
        to_assignee_id: row.to_assignee_id,
        field_name: row.field_name,
        from_text: row.from_text,
        to_text: row.to_text,
    }
}

// For activity types that don't carry any before/after data (created, comment).
pub async fn record(db: &PgPool, ticket_id: i64, user_id: i64, activity_type: ActivityType) -> Result<()> {
    sqlx::query(
        "INSERT INTO ticket_activity_log (ticket_id, user_id, activity_type, occurred_at) VALUES ($1, $2, $3, NOW())",
    )
    .bind(ticket_id)
    .bind(user_id)
    .bind(activity_type.to_string())
    .execute(db)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

pub async fn record_status_change(
    db: &PgPool,
    ticket_id: i64,
    user_id: i64,
    from_status_id: i64,
    to_status_id: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ticket_activity_log (ticket_id, user_id, activity_type, occurred_at, from_status_id, to_status_id) VALUES ($1, $2, 'status_change', NOW(), $3, $4)",
    )
    .bind(ticket_id)
    .bind(user_id)
    .bind(from_status_id)
    .bind(to_status_id)
    .execute(db)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

pub async fn record_assignee_change(
    db: &PgPool,
    ticket_id: i64,
    user_id: i64,
    from_assignee_id: Option<i64>,
    to_assignee_id: Option<i64>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ticket_activity_log (ticket_id, user_id, activity_type, occurred_at, from_assignee_id, to_assignee_id) VALUES ($1, $2, 'assignee_change', NOW(), $3, $4)",
    )
    .bind(ticket_id)
    .bind(user_id)
    .bind(from_assignee_id)
    .bind(to_assignee_id)
    .execute(db)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

pub async fn record_field_change(
    db: &PgPool,
    ticket_id: i64,
    user_id: i64,
    field_name: &str,
    from_text: Option<String>,
    to_text: Option<String>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ticket_activity_log (ticket_id, user_id, activity_type, occurred_at, field_name, from_text, to_text) VALUES ($1, $2, 'field_update', NOW(), $3, $4, $5)",
    )
    .bind(ticket_id)
    .bind(user_id)
    .bind(field_name)
    .bind(from_text)
    .bind(to_text)
    .execute(db)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

pub async fn list_for_ticket(db: &PgPool, ticket_id: i64, limit: i64, offset: i64) -> Result<Vec<TicketActivity>> {
    let sql = format!(
        "SELECT {ACTIVITY_COLUMNS} FROM ticket_activity_log WHERE ticket_id = $1 ORDER BY occurred_at DESC LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, ActivityRow>(&sql)
        .bind(ticket_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_activity).collect())
}

// All status_change rows for a ticket, oldest first - the full unpaginated
// history is needed to reconstruct how long it spent in each status (see
// tickets::duration::in_progress_seconds), unlike list_for_ticket which is
// paginated for the human-facing timeline view.
pub async fn list_status_changes_for_ticket(
    db: &PgPool,
    ticket_id: i64,
) -> Result<Vec<(chrono::DateTime<chrono::Utc>, i64, i64)>> {
    sqlx::query_as::<_, (chrono::DateTime<chrono::Utc>, i64, i64)>(
        "SELECT occurred_at, from_status_id, to_status_id FROM ticket_activity_log \
         WHERE ticket_id = $1 AND activity_type = 'status_change' ORDER BY occurred_at ASC",
    )
    .bind(ticket_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
}

pub async fn count_for_ticket(db: &PgPool, ticket_id: i64) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ticket_activity_log WHERE ticket_id = $1")
        .bind(ticket_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}

pub async fn list_for_user_on_date(db: &PgPool, user_id: i64, date: NaiveDate, tz: &str) -> Result<Vec<TicketActivity>> {
    let sql = format!(
        "SELECT {ACTIVITY_COLUMNS} FROM ticket_activity_log WHERE user_id = $1 AND (occurred_at AT TIME ZONE $3)::date = $2 ORDER BY occurred_at DESC"
    );
    sqlx::query_as::<_, ActivityRow>(&sql)
        .bind(user_id)
        .bind(date)
        .bind(tz)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_activity).collect())
}
