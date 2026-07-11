use chrono::{DateTime, Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::auth::User;
use crate::error::{AppError, Result};
use crate::users::repo::{row_to_user, UserRow};
use super::models::{ClosureState, Decision, DecisionRecord, LockReason};

const USER_COLUMNS: &str = "u.id, u.email, u.display_name, u.role, u.status, u.locale, u.avatar_updated_at, u.is_approver";

pub async fn is_approver_of(db: &PgPool, approver_id: i64, user_id: i64) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM user_approvers WHERE user_id = $1 AND approver_id = $2)",
    )
    .bind(user_id)
    .bind(approver_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;
    Ok(exists)
}

pub async fn list_subordinates(db: &PgPool, approver_id: i64) -> Result<Vec<User>> {
    let sql = format!(
        "SELECT {USER_COLUMNS} FROM user_approvers ua JOIN users u ON u.id = ua.user_id \
         WHERE ua.approver_id = $1 AND u.status != 'deleted' ORDER BY u.display_name"
    );
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(approver_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_user).collect())
}

pub async fn list_approvers_for_user(db: &PgPool, user_id: i64) -> Result<Vec<User>> {
    let sql = format!(
        "SELECT {USER_COLUMNS} FROM user_approvers ua JOIN users u ON u.id = ua.approver_id \
         WHERE ua.user_id = $1 AND u.status != 'deleted' ORDER BY u.display_name"
    );
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_user).collect())
}

pub async fn add_approver(db: &PgPool, user_id: i64, approver_id: i64, assigned_by_id: Option<i64>) -> Result<()> {
    let result = sqlx::query("INSERT INTO user_approvers (user_id, approver_id, assigned_by_id) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(approver_id)
        .bind(assigned_by_id)
        .execute(db)
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err))
            if db_err.constraint() == Some("user_approvers_user_id_approver_id_key") =>
        {
            Err(AppError::BadRequest("this user is already an approver for this user".to_string()))
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn remove_approver(db: &PgPool, user_id: i64, approver_id: i64) -> Result<()> {
    let result = sqlx::query("DELETE FROM user_approvers WHERE user_id = $1 AND approver_id = $2")
        .bind(user_id)
        .bind(approver_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

type DecisionRow = (String, String, i64, String, DateTime<Utc>);

fn row_to_decision(row: DecisionRow) -> DecisionRecord {
    let (decision_str, comment, approver_id, approver_name, decided_at) = row;
    DecisionRecord {
        decision: decision_str.parse().unwrap_or(Decision::Rejected),
        comment,
        approver_id,
        approver_name,
        decided_at,
    }
}

/// Full history for a single day, newest first - unfiltered, including
/// decisions superseded by a later resubmission or month reopen. Used for
/// the approver-facing audit view.
pub async fn list_decision_history(db: &PgPool, user_id: i64, date: NaiveDate) -> Result<Vec<DecisionRecord>> {
    sqlx::query_as::<_, DecisionRow>(
        "SELECT da.decision, da.comment, da.approver_id, u.display_name, da.decided_at
         FROM daily_approvals da JOIN users u ON u.id = da.approver_id
         WHERE da.user_id = $1 AND da.report_date = $2
         ORDER BY da.decided_at DESC",
    )
    .bind(user_id)
    .bind(date)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_decision).collect())
}

pub async fn latest_reopen_at(db: &PgPool, user_id: i64, year: i32, month: i32) -> Result<Option<DateTime<Utc>>> {
    let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT acted_at FROM monthly_closures
         WHERE user_id = $1 AND year = $2 AND month = $3 AND action = 'reopened'
         ORDER BY acted_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(year)
    .bind(month)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?;
    Ok(row.map(|(t,)| t))
}

/// The current effective decision for a day, or None if there isn't one yet
/// or the latest one has been superseded (report resubmitted after it, or
/// the month was reopened after it) - in which case the day is back to
/// "valid" (awaiting a fresh decision).
pub async fn get_current_decision(
    db: &PgPool,
    user_id: i64,
    date: NaiveDate,
    report_submitted_at: DateTime<Utc>,
) -> Result<Option<DecisionRecord>> {
    let latest = sqlx::query_as::<_, DecisionRow>(
        "SELECT da.decision, da.comment, da.approver_id, u.display_name, da.decided_at
         FROM daily_approvals da JOIN users u ON u.id = da.approver_id
         WHERE da.user_id = $1 AND da.report_date = $2
         ORDER BY da.decided_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?
    .map(row_to_decision);

    let Some(decision) = latest else { return Ok(None) };
    if decision.decided_at < report_submitted_at {
        return Ok(None);
    }
    if let Some(reopen_at) = latest_reopen_at(db, user_id, date.year(), date.month() as i32).await? {
        if decision.decided_at < reopen_at {
            return Ok(None);
        }
    }
    Ok(Some(decision))
}

/// Bulk equivalent of get_current_decision for a whole month, so the
/// monthly calendar view doesn't need one round-trip per day.
pub async fn list_current_decisions_for_month(
    db: &PgPool,
    user_id: i64,
    year: i32,
    month: u32,
) -> Result<HashMap<NaiveDate, DecisionRecord>> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::BadRequest("invalid year/month".to_string()))?;
    let end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| AppError::BadRequest("invalid year/month".to_string()))?;

    type MonthDecisionRow = (NaiveDate, String, String, i64, String, DateTime<Utc>);
    let decision_rows: Vec<MonthDecisionRow> = sqlx::query_as(
        "SELECT DISTINCT ON (da.report_date) da.report_date, da.decision, da.comment, da.approver_id, u.display_name, da.decided_at
         FROM daily_approvals da JOIN users u ON u.id = da.approver_id
         WHERE da.user_id = $1 AND da.report_date >= $2 AND da.report_date < $3
         ORDER BY da.report_date, da.decided_at DESC",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;

    let report_rows: Vec<(NaiveDate, DateTime<Utc>)> = sqlx::query_as(
        "SELECT report_date, submitted_at FROM daily_reports WHERE user_id = $1 AND report_date >= $2 AND report_date < $3",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;
    let submitted_at_by_date: HashMap<NaiveDate, DateTime<Utc>> = report_rows.into_iter().collect();

    let reopen_at = latest_reopen_at(db, user_id, year, month as i32).await?;

    let mut result = HashMap::new();
    for (date, decision_str, comment, approver_id, approver_name, decided_at) in decision_rows {
        let Some(&submitted_at) = submitted_at_by_date.get(&date) else { continue };
        if decided_at < submitted_at {
            continue;
        }
        if let Some(reopen_at) = reopen_at {
            if decided_at < reopen_at {
                continue;
            }
        }
        result.insert(
            date,
            DecisionRecord {
                decision: decision_str.parse().unwrap_or(Decision::Rejected),
                comment,
                approver_id,
                approver_name,
                decided_at,
            },
        );
    }
    Ok(result)
}

pub async fn insert_decision(
    db: &PgPool,
    user_id: i64,
    date: NaiveDate,
    approver_id: i64,
    approver_name: &str,
    decision: Decision,
    comment: &str,
) -> Result<DecisionRecord> {
    let (decided_at,): (DateTime<Utc>,) = sqlx::query_as(
        "INSERT INTO daily_approvals (user_id, report_date, decision, comment, approver_id)
         VALUES ($1, $2, $3, $4, $5) RETURNING decided_at",
    )
    .bind(user_id)
    .bind(date)
    .bind(decision.to_string())
    .bind(comment)
    .bind(approver_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;

    Ok(DecisionRecord {
        decision,
        comment: comment.to_string(),
        approver_id,
        approver_name: approver_name.to_string(),
        decided_at,
    })
}

pub async fn get_closure_state(db: &PgPool, user_id: i64, year: i32, month: i32) -> Result<ClosureState> {
    let row: Option<(String, i64, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT mc.action, mc.actor_id, u.display_name, mc.acted_at
         FROM monthly_closures mc JOIN users u ON u.id = mc.actor_id
         WHERE mc.user_id = $1 AND mc.year = $2 AND mc.month = $3
         ORDER BY mc.acted_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(year)
    .bind(month)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?;

    Ok(match row {
        Some((action, actor_id, actor_name, acted_at)) if action == "closed" => ClosureState {
            closed: true,
            closed_by: Some(actor_id),
            closed_by_name: Some(actor_name),
            closed_at: Some(acted_at),
        },
        _ => ClosureState { closed: false, closed_by: None, closed_by_name: None, closed_at: None },
    })
}

pub async fn is_month_closed(db: &PgPool, user_id: i64, year: i32, month: i32) -> Result<bool> {
    Ok(get_closure_state(db, user_id, year, month).await?.closed)
}

pub async fn insert_closure_action(
    db: &PgPool,
    user_id: i64,
    year: i32,
    month: i32,
    actor_id: i64,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO monthly_closures (user_id, year, month, action, actor_id) VALUES ($1, $2, $3, $4, $5)")
        .bind(user_id)
        .bind(year)
        .bind(month)
        .bind(action)
        .bind(actor_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// Whether a day's report can currently be edited - MonthClosed takes
/// priority over Approved in the reported reason since it's the broader lock.
pub async fn check_report_lock(
    db: &PgPool,
    user_id: i64,
    date: NaiveDate,
    report_submitted_at: DateTime<Utc>,
) -> Result<Option<LockReason>> {
    if is_month_closed(db, user_id, date.year(), date.month() as i32).await? {
        return Ok(Some(LockReason::MonthClosed));
    }
    if let Some(decision) = get_current_decision(db, user_id, date, report_submitted_at).await? {
        if decision.decision == Decision::Approved {
            return Ok(Some(LockReason::Approved));
        }
    }
    Ok(None)
}
