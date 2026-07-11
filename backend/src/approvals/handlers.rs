use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    auth::extractors::AuthUser,
    daily_reports,
    error::{AppError, Result},
    state::AppState,
    work_sessions::{self, models::DayStatus},
};
use super::{
    models::{ClosureState, DecideRequest, Decision, DecisionRecord},
    repo,
};

async fn require_approver_of(db: &sqlx::PgPool, approver_id: i64, user_id: i64) -> Result<()> {
    if repo::is_approver_of(db, approver_id, user_id).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

#[derive(serde::Serialize)]
pub struct SubordinateSummary {
    #[serde(flatten)]
    user: crate::auth::User,
    // Days in the current month that are Valid (report submitted, awaiting
    // a decision) - lets the approver see at a glance who needs attention.
    pending_count: i64,
}

pub async fn list_subordinates(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<SubordinateSummary>>> {
    let subordinates = repo::list_subordinates(&state.db, user.id).await?;

    let mut result = Vec::with_capacity(subordinates.len());
    for subordinate in subordinates {
        // "Current month" is relative to each subordinate's own timezone,
        // not the approver's - they may not match.
        let tz = crate::users::repo::resolve_timezone(&state.db, subordinate.id).await?;
        let today = crate::time::today(&tz);
        let days =
            work_sessions::repo::get_monthly_summaries(&state.db, subordinate.id, today.year(), today.month(), &tz)
                .await?;
        let pending_count = days.iter().filter(|d| d.status == DayStatus::Valid).count() as i64;
        result.push(SubordinateSummary { user: subordinate, pending_count });
    }

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct MonthlyQuery {
    year: i32,
    month: u32,
}

#[derive(serde::Serialize)]
pub struct MonthlyReviewResponse {
    days: Vec<work_sessions::models::DaySummary>,
    closure: ClosureState,
}

pub async fn monthly(
    State(state): State<Arc<AppState>>,
    AuthUser(approver): AuthUser,
    Path(user_id): Path<i64>,
    Query(query): Query<MonthlyQuery>,
) -> Result<Json<MonthlyReviewResponse>> {
    require_approver_of(&state.db, approver.id, user_id).await?;

    let tz = crate::users::repo::resolve_timezone(&state.db, user_id).await?;
    let days = work_sessions::repo::get_monthly_summaries(&state.db, user_id, query.year, query.month, &tz).await?;
    let closure = repo::get_closure_state(&state.db, user_id, query.year, query.month as i32).await?;
    Ok(Json(MonthlyReviewResponse { days, closure }))
}

#[derive(Debug, Deserialize)]
pub struct UserDatePath {
    user_id: i64,
    date: NaiveDate,
}

#[derive(serde::Serialize)]
pub struct DayReviewResponse {
    report: Option<daily_reports::models::DailyReport>,
    history: Vec<DecisionRecord>,
    status: Option<DayStatus>,
}

pub async fn get_day(
    State(state): State<Arc<AppState>>,
    AuthUser(approver): AuthUser,
    Path(UserDatePath { user_id, date }): Path<UserDatePath>,
) -> Result<Json<DayReviewResponse>> {
    require_approver_of(&state.db, approver.id, user_id).await?;

    let report = daily_reports::repo::get_report(&state.db, user_id, date).await?;
    let history = repo::list_decision_history(&state.db, user_id, date).await?;
    let tz = crate::users::repo::resolve_timezone(&state.db, user_id).await?;
    let status = work_sessions::repo::get_day_summary(&state.db, user_id, date, &tz)
        .await?
        .map(|s| s.status);

    Ok(Json(DayReviewResponse { report, history, status }))
}

pub async fn decide(
    State(state): State<Arc<AppState>>,
    AuthUser(approver): AuthUser,
    Path(UserDatePath { user_id, date }): Path<UserDatePath>,
    Json(payload): Json<DecideRequest>,
) -> Result<Json<DecisionRecord>> {
    require_approver_of(&state.db, approver.id, user_id).await?;

    // A closed month must keep every day Approved (that's the precondition
    // for closing it), so revoking one is only possible after reopening the
    // month - which itself reverts every day back to Valid anyway.
    if repo::is_month_closed(&state.db, user_id, date.year(), date.month() as i32).await? {
        return Err(AppError::BadRequest(
            "this month has been closed and can no longer be edited".to_string(),
        ));
    }

    let tz = crate::users::repo::resolve_timezone(&state.db, user_id).await?;
    let day = work_sessions::repo::get_day_summary(&state.db, user_id, date, &tz)
        .await?
        .ok_or_else(|| AppError::BadRequest("this day has no work sessions to decide on".to_string()))?;

    // Normally a decision can only be made while the day is Valid (awaiting
    // one). The one exception: an Approved day can be revoked by rejecting
    // it - this is the only way to undo a mistaken approval short of
    // reopening the whole month.
    let can_decide = match day.status {
        DayStatus::Valid => true,
        DayStatus::Approved => payload.decision == Decision::Rejected,
        _ => false,
    };
    if !can_decide {
        return Err(AppError::BadRequest(
            "this day is not currently awaiting a decision".to_string(),
        ));
    }

    let record = repo::insert_decision(
        &state.db,
        user_id,
        date,
        approver.id,
        &approver.display_name,
        payload.decision,
        &payload.comment,
    )
    .await?;

    crate::notifications::repo::notify_report_decision(
        &state.db,
        user_id,
        approver.id,
        date,
        payload.decision == Decision::Approved,
    )
    .await?;

    Ok(Json(record))
}

#[derive(Debug, Deserialize)]
pub struct UserMonthPath {
    user_id: i64,
    year: i32,
    month: u32,
}

pub async fn close_month(
    State(state): State<Arc<AppState>>,
    AuthUser(approver): AuthUser,
    Path(UserMonthPath { user_id, year, month }): Path<UserMonthPath>,
) -> Result<Json<ClosureState>> {
    require_approver_of(&state.db, approver.id, user_id).await?;

    if repo::is_month_closed(&state.db, user_id, year, month as i32).await? {
        return Err(AppError::BadRequest("this month is already closed".to_string()));
    }

    let tz = crate::users::repo::resolve_timezone(&state.db, user_id).await?;
    let days = work_sessions::repo::get_monthly_summaries(&state.db, user_id, year, month, &tz).await?;
    if days.iter().any(|d| d.status != DayStatus::Approved) {
        return Err(AppError::BadRequest(
            "every day with work sessions this month must be approved before it can be closed".to_string(),
        ));
    }

    repo::insert_closure_action(&state.db, user_id, year, month as i32, approver.id, "closed").await?;
    let closure = repo::get_closure_state(&state.db, user_id, year, month as i32).await?;
    Ok(Json(closure))
}

pub async fn reopen_month(
    State(state): State<Arc<AppState>>,
    AuthUser(approver): AuthUser,
    Path(UserMonthPath { user_id, year, month }): Path<UserMonthPath>,
) -> Result<Json<ClosureState>> {
    require_approver_of(&state.db, approver.id, user_id).await?;

    if !repo::is_month_closed(&state.db, user_id, year, month as i32).await? {
        return Err(AppError::BadRequest("this month is not closed".to_string()));
    }

    repo::insert_closure_action(&state.db, user_id, year, month as i32, approver.id, "reopened").await?;
    let closure = repo::get_closure_state(&state.db, user_id, year, month as i32).await?;
    Ok(Json(closure))
}
