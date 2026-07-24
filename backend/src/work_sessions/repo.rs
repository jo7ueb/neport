use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::approvals::models::{Decision, DecisionRecord};
use crate::error::{AppError, Result};
use super::models::{DayStatus, DaySummary, WorkSession};

type SessionRow = (i64, i64, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>);

fn row_to_session(row: SessionRow) -> WorkSession {
    let (id, user_id, started_at, ended_at, created_at, updated_at) = row;
    WorkSession {
        id,
        user_id,
        started_at,
        ended_at,
        created_at,
        updated_at,
    }
}

pub async fn get_open_session(db: &PgPool, user_id: i64) -> Result<Option<WorkSession>> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, started_at, ended_at, created_at, updated_at FROM work_sessions WHERE user_id = $1 AND ended_at IS NULL"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_session))
}

pub async fn start_session(db: &PgPool, user_id: i64, access_log_id: i64) -> Result<WorkSession> {
    let result = sqlx::query_as::<_, SessionRow>(
        "INSERT INTO work_sessions (user_id, started_at, start_access_log_id) VALUES ($1, NOW(), $2) RETURNING id, user_id, started_at, ended_at, created_at, updated_at"
    )
    .bind(user_id)
    .bind(access_log_id)
    .fetch_one(db)
    .await;

    match result {
        Ok(row) => Ok(row_to_session(row)),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint() == Some("idx_work_sessions_one_open_per_user") {
                Err(AppError::BadRequest("work is already in progress".to_string()))
            } else {
                Err(AppError::Database(sqlx::Error::Database(db_err)))
            }
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn get_session(db: &PgPool, id: i64) -> Result<Option<WorkSession>> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, started_at, ended_at, created_at, updated_at FROM work_sessions WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_session))
}

// Callers resolve started_at/ended_at to their final values (applying any
// COALESCE-with-existing logic and validating started_at < ended_at)
// themselves before calling this, since that also drives what gets written
// to the audit log.
pub async fn update_session(
    db: &PgPool,
    id: i64,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Result<WorkSession> {
    sqlx::query_as::<_, SessionRow>(
        "UPDATE work_sessions SET started_at = $2, ended_at = $3, updated_at = NOW() WHERE id = $1 \
         RETURNING id, user_id, started_at, ended_at, created_at, updated_at"
    )
    .bind(id)
    .bind(started_at)
    .bind(ended_at)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(row_to_session)
}

pub async fn end_session(db: &PgPool, user_id: i64, access_log_id: i64) -> Result<WorkSession> {
    let open = get_open_session(db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("no work session is currently in progress".to_string()))?;

    sqlx::query_as::<_, SessionRow>(
        "UPDATE work_sessions SET ended_at = NOW(), updated_at = NOW(), end_access_log_id = $2 WHERE id = $1 RETURNING id, user_id, started_at, ended_at, created_at, updated_at"
    )
    .bind(open.id)
    .bind(access_log_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(row_to_session)
}

pub async fn list_sessions_for_date(db: &PgPool, user_id: i64, date: NaiveDate, tz: &str) -> Result<Vec<WorkSession>> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, started_at, ended_at, created_at, updated_at FROM work_sessions \
         WHERE user_id = $1 AND (started_at AT TIME ZONE $3)::date = $2 ORDER BY started_at ASC"
    )
    .bind(user_id)
    .bind(date)
    .bind(tz)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_session).collect())
}

pub async fn list_sessions_for_month(db: &PgPool, user_id: i64, year: i32, month: u32, tz: &str) -> Result<Vec<WorkSession>> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::BadRequest("invalid year/month".to_string()))?;
    let end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| AppError::BadRequest("invalid year/month".to_string()))?;

    // Month boundaries are computed as local midnight (in the user's
    // timezone) converted to the equivalent UTC instant, so a session near
    // midnight on the first/last day of the month lands in the right month
    // regardless of how far the user's timezone is offset from UTC.
    sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, started_at, ended_at, created_at, updated_at FROM work_sessions WHERE user_id = $1 AND started_at >= $2 AND started_at < $3 ORDER BY started_at ASC"
    )
    .bind(user_id)
    .bind(crate::time::local_midnight_utc(start, tz))
    .bind(crate::time::local_midnight_utc(end, tz))
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_session).collect())
}

fn summarize(
    date: NaiveDate,
    sessions: Vec<WorkSession>,
    report_submitted_at: Option<DateTime<Utc>>,
    decision: Option<DecisionRecord>,
) -> DaySummary {
    let first_started_at = sessions[0].started_at;
    let all_closed = sessions.iter().all(|s| s.ended_at.is_some());
    let last_ended_at = if all_closed {
        sessions.iter().filter_map(|s| s.ended_at).max()
    } else {
        None
    };

    let status = if !all_closed {
        DayStatus::InProgress
    } else if report_submitted_at.is_none() {
        DayStatus::Invalid
    } else {
        match decision.as_ref().map(|d| d.decision) {
            Some(Decision::Approved) => DayStatus::Approved,
            Some(Decision::Rejected) => DayStatus::Rejected,
            None => DayStatus::Valid,
        }
    };

    DaySummary {
        date,
        sessions,
        first_started_at,
        last_ended_at,
        status,
        decision: if matches!(status, DayStatus::Approved | DayStatus::Rejected) {
            decision
        } else {
            None
        },
    }
}

pub async fn get_day_summary(db: &PgPool, user_id: i64, date: NaiveDate, tz: &str) -> Result<Option<DaySummary>> {
    let sessions = list_sessions_for_date(db, user_id, date, tz).await?;
    if sessions.is_empty() {
        return Ok(None);
    }

    let report = crate::daily_reports::repo::get_report(db, user_id, date).await?;
    let decision = match &report {
        Some(r) => crate::approvals::repo::get_current_decision(db, user_id, date, r.submitted_at).await?,
        None => None,
    };

    Ok(Some(summarize(date, sessions, report.map(|r| r.submitted_at), decision)))
}

pub async fn get_monthly_summaries(db: &PgPool, user_id: i64, year: i32, month: u32, tz: &str) -> Result<Vec<DaySummary>> {
    let sessions = list_sessions_for_month(db, user_id, year, month, tz).await?;

    let mut by_date: HashMap<NaiveDate, Vec<WorkSession>> = HashMap::new();
    for session in sessions {
        by_date.entry(crate::time::local_date(session.started_at, tz)).or_default().push(session);
    }

    // Needs every report date the user has ever submitted (not a paginated
    // page) to correctly determine day validity, so request the internal
    // "fetch everything" ceiling.
    let reported_dates: HashMap<NaiveDate, DateTime<Utc>> = crate::daily_reports::repo::list_reports_for_user(
        db,
        user_id,
        crate::pagination::FETCH_ALL_LIMIT,
        0,
    )
        .await?
        .into_iter()
        .map(|r| (r.report_date, r.submitted_at))
        .collect();

    let decisions = crate::approvals::repo::list_current_decisions_for_month(db, user_id, year, month).await?;

    let mut summaries: Vec<DaySummary> = by_date
        .into_iter()
        .map(|(date, sessions)| {
            let report_submitted_at = reported_dates.get(&date).copied();
            let decision = decisions.get(&date).cloned();
            summarize(date, sessions, report_submitted_at, decision)
        })
        .collect();

    summaries.sort_by_key(|s| s.date);
    Ok(summaries)
}

