use chrono::NaiveDate;
use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::tickets::models::Ticket;
use crate::tickets::repo::{row_to_ticket, TicketRow, TICKET_SELECT_COLUMNS};
use super::models::DailyReport;

type ReportRow = (i64, i64, NaiveDate, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>);

fn row_to_report(row: ReportRow) -> DailyReport {
    let (id, user_id, report_date, achievement, leftover, keep, problem, try_, submitted_at, created_at, updated_at) = row;
    DailyReport {
        id,
        user_id,
        report_date,
        achievement,
        leftover,
        keep,
        problem,
        try_,
        submitted_at,
        created_at,
        updated_at,
    }
}

pub async fn get_report(db: &PgPool, user_id: i64, date: NaiveDate) -> Result<Option<DailyReport>> {
    sqlx::query_as::<_, ReportRow>(
        "SELECT id, user_id, report_date, achievement, leftover, keep, problem, try, submitted_at, created_at, updated_at FROM daily_reports WHERE user_id = $1 AND report_date = $2"
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_report))
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_report(
    db: &PgPool,
    user_id: i64,
    date: NaiveDate,
    achievement: Option<&str>,
    leftover: Option<&str>,
    keep: Option<&str>,
    problem: Option<&str>,
    try_: Option<&str>,
) -> Result<DailyReport> {
    sqlx::query_as::<_, ReportRow>(
        "INSERT INTO daily_reports (user_id, report_date, achievement, leftover, keep, problem, try, submitted_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
         ON CONFLICT (user_id, report_date) DO UPDATE SET
            achievement = EXCLUDED.achievement,
            leftover = EXCLUDED.leftover,
            keep = EXCLUDED.keep,
            problem = EXCLUDED.problem,
            try = EXCLUDED.try,
            submitted_at = NOW(),
            updated_at = NOW()
         RETURNING id, user_id, report_date, achievement, leftover, keep, problem, try, submitted_at, created_at, updated_at"
    )
    .bind(user_id)
    .bind(date)
    .bind(achievement)
    .bind(leftover)
    .bind(keep)
    .bind(problem)
    .bind(try_)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(row_to_report)
}

pub async fn list_reports_for_user(db: &PgPool, user_id: i64, limit: i64, offset: i64) -> Result<Vec<DailyReport>> {
    sqlx::query_as::<_, ReportRow>(
        "SELECT id, user_id, report_date, achievement, leftover, keep, problem, try, submitted_at, created_at, updated_at FROM daily_reports WHERE user_id = $1 ORDER BY report_date DESC LIMIT $2 OFFSET $3"
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_report).collect())
}

pub async fn count_reports_for_user(db: &PgPool, user_id: i64) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM daily_reports WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}

pub async fn list_touched_tickets(db: &PgPool, user_id: i64, date: NaiveDate, tz: &str) -> Result<Vec<Ticket>> {
    let sql = format!(
        "SELECT DISTINCT ON (t.id) {TICKET_SELECT_COLUMNS}
         FROM tickets t
         JOIN workflow_statuses ws ON t.status_id = ws.id
         JOIN ticket_activity_log tal ON tal.ticket_id = t.id
         WHERE tal.user_id = $1 AND (tal.occurred_at AT TIME ZONE $3)::date = $2
         ORDER BY t.id"
    );
    sqlx::query_as::<_, TicketRow>(&sql)
        .bind(user_id)
        .bind(date)
        .bind(tz)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_ticket).collect())
}
