use axum::{extract::{Path, Query, State}, Json};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;

use crate::{approvals, auth::extractors::AuthUser, error::{AppError, Result}, pagination::{PageQuery, Paginated}, state::AppState, tickets::models::Ticket};
use super::{models::{DailyReport, DailyReportView}, repo};

pub async fn get_today_tickets(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<Ticket>>> {
    let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
    let today = crate::time::today(&tz);
    let tickets = repo::list_touched_tickets(&state.db, user.id, today, &tz).await?;
    Ok(Json(tickets))
}

pub async fn get_report(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(date): Path<NaiveDate>,
) -> Result<Json<DailyReportView>> {
    let report = repo::get_report(&state.db, user.id, date)
        .await?
        .ok_or(AppError::NotFound)?;
    let lock_reason = approvals::repo::check_report_lock(&state.db, user.id, date, report.submitted_at).await?;
    let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
    let touched_tickets = repo::list_touched_tickets(&state.db, user.id, date, &tz).await?;
    Ok(Json(DailyReportView { report, locked: lock_reason.is_some(), lock_reason, touched_tickets }))
}

#[derive(Debug, Deserialize)]
pub struct SubmitReportRequest {
    pub report_date: Option<NaiveDate>,
    pub achievement: Option<String>,
    pub leftover: Option<String>,
    pub keep: Option<String>,
    pub problem: Option<String>,
    #[serde(rename = "try")]
    pub try_: Option<String>,
}

pub async fn submit_report(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(payload): Json<SubmitReportRequest>,
) -> Result<Json<DailyReport>> {
    let date = match payload.report_date {
        Some(d) => d,
        None => {
            let tz = crate::users::repo::resolve_timezone(&state.db, user.id).await?;
            crate::time::today(&tz)
        }
    };

    if let Some(existing) = repo::get_report(&state.db, user.id, date).await? {
        match approvals::repo::check_report_lock(&state.db, user.id, date, existing.submitted_at).await? {
            Some(approvals::models::LockReason::Approved) => {
                return Err(AppError::BadRequest(
                    "this day's report has already been approved and can no longer be edited".to_string(),
                ));
            }
            Some(approvals::models::LockReason::MonthClosed) => {
                return Err(AppError::BadRequest(
                    "this month has been closed and can no longer be edited".to_string(),
                ));
            }
            None => {}
        }
    }

    let report = repo::upsert_report(
        &state.db,
        user.id,
        date,
        payload.achievement.as_deref(),
        payload.leftover.as_deref(),
        payload.keep.as_deref(),
        payload.problem.as_deref(),
        payload.try_.as_deref(),
    )
    .await?;

    Ok(Json(report))
}

pub async fn list_reports(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(page): Query<PageQuery>,
) -> Result<Json<Paginated<DailyReport>>> {
    let reports = repo::list_reports_for_user(&state.db, user.id, page.limit(), page.offset()).await?;
    let total = repo::count_reports_for_user(&state.db, user.id).await?;
    Ok(Json(Paginated::new(reports, total, &page)))
}
