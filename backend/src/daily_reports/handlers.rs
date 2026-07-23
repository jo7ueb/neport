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

    let existing = repo::get_report(&state.db, user.id, date).await?;
    if let Some(existing) = &existing {
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

    let old_text = report_mention_text(existing.as_ref().map(|r| (
        r.achievement.as_deref(), r.leftover.as_deref(), r.keep.as_deref(), r.problem.as_deref(), r.try_.as_deref(),
    )).unwrap_or_default());
    let new_text = report_mention_text((
        payload.achievement.as_deref(), payload.leftover.as_deref(), payload.keep.as_deref(),
        payload.problem.as_deref(), payload.try_.as_deref(),
    ));

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

    for mentioned_id in crate::mentions::newly_mentioned(&old_text, &new_text) {
        crate::notifications::repo::notify_mentioned_in_report(&state.db, mentioned_id, user.id, date).await?;
    }

    Ok(Json(report))
}

// Concatenates the report's five free-text KPT fields into one string for
// mention extraction - which field a mention lives in doesn't matter, only
// whether it's new since the last save (see mentions::newly_mentioned).
fn report_mention_text(fields: (Option<&str>, Option<&str>, Option<&str>, Option<&str>, Option<&str>)) -> String {
    let (achievement, leftover, keep, problem, try_) = fields;
    [achievement, leftover, keep, problem, try_]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
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
