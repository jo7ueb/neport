use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::{approvals, auth::extractors::AuthUser, error::{AppError, Result}, state::AppState};
use super::{
    models::{DaySummary, WorkSession},
    repo,
};

async fn ensure_current_month_open(state: &AppState, user_id: i64) -> Result<()> {
    let tz = crate::users::repo::resolve_timezone(&state.db, user_id).await?;
    let today = crate::time::today(&tz);
    ensure_month_editable(state, user_id, today).await
}

// Shared by the current-month start/end guard above and by session editing
// below, which additionally needs to check the (possibly past) date the
// session actually falls on, not just "today".
async fn ensure_month_editable(state: &AppState, user_id: i64, date: NaiveDate) -> Result<()> {
    if approvals::repo::is_month_closed(&state.db, user_id, date.year(), date.month() as i32).await? {
        return Err(AppError::BadRequest(
            "this month has been closed and can no longer be edited".to_string(),
        ));
    }
    Ok(())
}

// Blocks editing a session on a day whose report has already been approved,
// on top of the month-closed check every write already goes through -
// otherwise an edit could silently invalidate hours an approver already
// signed off on. Mirrors daily_reports::handlers::submit_report's use of the
// same lock.
async fn ensure_session_editable(state: &AppState, user_id: i64, date: NaiveDate) -> Result<()> {
    ensure_month_editable(state, user_id, date).await?;
    if let Some(report) = crate::daily_reports::repo::get_report(&state.db, user_id, date).await? {
        if let Some(approvals::models::LockReason::Approved) =
            approvals::repo::check_report_lock(&state.db, user_id, date, report.submitted_at).await?
        {
            return Err(AppError::BadRequest(
                "this day's report has already been approved and can no longer be edited".to_string(),
            ));
        }
    }
    Ok(())
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
) -> Result<(StatusCode, Json<WorkSession>)> {
    ensure_current_month_open(&state, user.id).await?;
    let access_log_id = crate::audit::access_log::record(&state.db, user.id, ip.as_deref()).await?;
    let session = repo::start_session(&state.db, user.id, access_log_id).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn end(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
) -> Result<Json<WorkSession>> {
    ensure_current_month_open(&state, user.id).await?;
    let access_log_id = crate::audit::access_log::record(&state.db, user.id, ip.as_deref()).await?;
    let session = repo::end_session(&state.db, user.id, access_log_id).await?;
    Ok(Json(session))
}

pub async fn current(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<WorkSession>> {
    let session = repo::get_open_session(&state.db, user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(session))
}

pub async fn today(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<DaySummary>> {
    let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
    let today = crate::time::today(&tz);
    let summary = repo::get_day_summary(&state.db, user.id, today, &tz)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(summary))
}

#[derive(Debug, Deserialize)]
pub struct DayQuery {
    date: NaiveDate,
}

pub async fn day(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<DayQuery>,
) -> Result<Json<DaySummary>> {
    let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
    let summary = repo::get_day_summary(&state.db, user.id, query.date, &tz)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(summary))
}

#[derive(Debug, Deserialize)]
pub struct MonthlyQuery {
    year: i32,
    month: u32,
}

#[derive(serde::Serialize)]
pub struct MonthlyResponse {
    days: Vec<DaySummary>,
    closure: approvals::models::ClosureState,
}

pub async fn monthly(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<MonthlyQuery>,
) -> Result<Json<MonthlyResponse>> {
    let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
    let days = repo::get_monthly_summaries(&state.db, user.id, query.year, query.month, &tz).await?;
    let closure = approvals::repo::get_closure_state(&state.db, user.id, query.year, query.month as i32).await?;
    Ok(Json(MonthlyResponse { days, closure }))
}

// Both fields are optional and left unchanged when omitted (like other
// PATCH endpoints in this codebase, e.g. milestones::UpdateMilestoneRequest) -
// this is what lets the same endpoint both fix a typo'd started_at and, for
// the "forgot to press end" case this was built for, supply the missing
// ended_at on an otherwise-still-open session.
#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

fn describe_session(started_at: DateTime<Utc>, ended_at: Option<DateTime<Utc>>) -> String {
    match ended_at {
        Some(ended_at) => format!("{started_at} - {ended_at}"),
        None => format!("{started_at} - (in progress)"),
    }
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateSessionRequest>,
) -> Result<Json<WorkSession>> {
    let existing = repo::get_session(&state.db, id).await?.ok_or(AppError::NotFound)?;
    if existing.user_id != user.id {
        return Err(AppError::Forbidden);
    }

    let new_started_at = payload.started_at.unwrap_or(existing.started_at);
    let new_ended_at = payload.ended_at.or(existing.ended_at);
    if let Some(ended_at) = new_ended_at {
        if new_started_at >= ended_at {
            return Err(AppError::BadRequest("started_at must be before ended_at".to_string()));
        }
    }

    let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
    let old_date = crate::time::local_date(existing.started_at, &tz);
    let new_date = crate::time::local_date(new_started_at, &tz);
    ensure_session_editable(&state, user.id, old_date).await?;
    if new_date != old_date {
        ensure_session_editable(&state, user.id, new_date).await?;
    }

    let updated = repo::update_session(&state.db, id, new_started_at, new_ended_at).await?;

    crate::audit::repo::record(
        &state.db,
        user.id,
        "work_session_edited",
        "work_session",
        Some(id),
        Some(&describe_session(existing.started_at, existing.ended_at)),
        Some(&describe_session(updated.started_at, updated.ended_at)),
        ip.as_deref(),
    )
    .await?;

    Ok(Json(updated))
}
