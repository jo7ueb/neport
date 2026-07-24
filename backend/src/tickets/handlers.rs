use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    access::{self, Access},
    activity_log::{self, ActivityType},
    auth::extractors::AuthUser,
    auth::UserRole,
    error::{AppError, Result},
    pagination::{PageQuery, Paginated},
    state::AppState,
};
use crate::workflows::models::StatusCategory;
use super::{
    models::{Ticket, TicketFilter, TicketNode},
    repo,
};

#[derive(Debug, Deserialize)]
pub struct CreateTicketRequest {
    pub project_id: i64,
    pub parent_ticket_id: Option<i64>,
    pub assignee_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub due_date: Option<String>,
    pub milestone_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListTicketsQuery {
    pub assignee_id: Option<i64>,
    pub status_id: Option<i64>,
    pub category: Option<StatusCategory>,
    pub overdue: Option<bool>,
    pub sort: Option<super::models::TicketSort>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchTicketsQuery {
    pub q: Option<String>,
}

pub async fn search_tickets(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<SearchTicketsQuery>,
) -> Result<Json<Vec<Ticket>>> {
    let q = query.q.unwrap_or_default();
    let is_admin = user.role == UserRole::Admin;
    let tickets = repo::search_tickets(&state.db, &q, 8, user.id, is_admin).await?;
    Ok(Json(tickets))
}

pub async fn get_ticket(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Ticket>> {
    let ticket = repo::get_ticket(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    access::check_project_access(&state.db, &user, ticket.project_id, Access::Read).await?;
    Ok(Json(ticket))
}

pub async fn list_children(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Ticket>>> {
    let project_id = access::project_id_for_ticket(&state.db, id).await?;
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let children = repo::list_children(&state.db, id).await?;
    Ok(Json(children))
}

pub async fn list_tickets(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(project_id): Path<i64>,
    Query(query): Query<ListTicketsQuery>,
) -> Result<Json<Paginated<Ticket>>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let page = PageQuery { page: query.page, per_page: query.per_page };
    let filter = TicketFilter {
        assignee_id: query.assignee_id,
        status_id: query.status_id,
        category: query.category,
        overdue: query.overdue.unwrap_or(false),
        sort: query.sort.unwrap_or_default(),
    };
    let tickets = repo::list_tickets_for_project(&state.db, project_id, &filter, page.limit(), page.offset()).await?;
    let total = repo::count_tickets_for_project(&state.db, project_id, &filter).await?;
    Ok(Json(Paginated::new(tickets, total, &page)))
}

pub async fn get_ticket_tree(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(project_id): Path<i64>,
) -> Result<Json<Vec<TicketNode>>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let tree = repo::get_ticket_tree(&state.db, project_id).await?;
    Ok(Json(tree))
}

pub async fn create_ticket(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Json(payload): Json<CreateTicketRequest>,
) -> Result<(StatusCode, Json<Ticket>)> {
    access::check_project_access(&state.db, &user, payload.project_id, Access::Write).await?;
    let start_date = payload
        .start_date
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = payload
        .end_date
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let due_date = payload
        .due_date
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let ticket = repo::create_ticket(
        &state.db,
        payload.project_id,
        payload.parent_ticket_id,
        user.id,
        payload.assignee_id,
        &payload.title,
        payload.description.as_deref(),
        start_date,
        end_date,
        due_date,
        payload.milestone_id,
    )
    .await?;

    let access_log_id = crate::audit::access_log::record(&state.db, user.id, ip.as_deref()).await?;
    activity_log::repo::record(&state.db, ticket.id, user.id, ActivityType::Created, access_log_id).await?;

    if let Some(assignee_id) = ticket.assignee_id {
        crate::notifications::repo::notify_ticket_assigned(&state.db, assignee_id, user.id, ticket.id).await?;
    }

    for mentioned_id in crate::mentions::extract_mentioned_user_ids(payload.description.as_deref().unwrap_or("")) {
        crate::notifications::repo::notify_mentioned_in_ticket(&state.db, mentioned_id, user.id, ticket.id).await?;
    }

    Ok((StatusCode::CREATED, Json(ticket)))
}

pub async fn update_ticket(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Ticket>> {
    let before = repo::get_ticket(&state.db, id).await?.ok_or(AppError::NotFound)?;
    access::check_project_access(&state.db, &user, before.project_id, Access::Write).await?;

    // Moving a ticket into a "confirmed"-category status (closing it after
    // review) is a project manager's call, not the assignee's - "done" just
    // means whoever did the work says it's finished; "confirmed" means a
    // manager verified that. Any other status change only needs Write.
    if let Some(new_status_id) = payload.get("status_id").and_then(|v| v.as_i64()) {
        if new_status_id != before.status.id {
            let new_category = crate::workflows::repo::get_status_category(&state.db, new_status_id).await?;
            if new_category == StatusCategory::Confirmed {
                access::check_project_access(&state.db, &user, before.project_id, Access::Manage).await?;
            }
        }
    }

    if let Some(parent_val) = payload.get("parent_ticket_id") {
        if let Some(new_parent_id) = parent_val.as_i64() {
            if new_parent_id == id {
                return Err(AppError::BadRequest(
                    "a ticket cannot be its own parent".into(),
                ));
            }

            if repo::would_create_cycle(&state.db, id, new_parent_id).await? {
                return Err(AppError::BadRequest(
                    "this move would create a cycle in the ticket hierarchy".into(),
                ));
            }
        }
    }

    let update = repo::build_ticket_update(&payload)?;
    let ticket = repo::update_ticket(&state.db, id, update).await?;

    let access_log_id = crate::audit::access_log::record(&state.db, user.id, ip.as_deref()).await?;
    record_update_activity(&state, id, user.id, &before, &ticket, access_log_id).await?;

    Ok(Json(ticket))
}

const DESCRIPTION_PREVIEW_LEN: usize = 60;

fn truncate_text(s: &str) -> String {
    if s.chars().count() <= DESCRIPTION_PREVIEW_LEN {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(DESCRIPTION_PREVIEW_LEN).collect();
        format!("{truncated}…")
    }
}

async fn milestone_text(state: &AppState, id: Option<i64>) -> Result<Option<String>> {
    match id {
        Some(id) => Ok(crate::milestones::repo::get_milestone(&state.db, id).await?.map(|m| m.name)),
        None => Ok(None),
    }
}

async fn parent_ticket_text(state: &AppState, id: Option<i64>) -> Result<Option<String>> {
    match id {
        Some(id) => Ok(repo::get_ticket(&state.db, id).await?.map(|t| format!("#{} {}", t.id, t.title))),
        None => Ok(None),
    }
}

// Classifies what changed between the ticket's before/after state and
// records one activity log entry per kind of change. Status/assignee
// changes carry the from/to ids; every other field that changed gets its
// own "field_update" row with a before/after text snapshot, so the
// timeline can show what actually changed rather than a vague "updated".
async fn record_update_activity(
    state: &AppState,
    ticket_id: i64,
    user_id: i64,
    before: &Ticket,
    after: &Ticket,
    access_log_id: i64,
) -> Result<()> {
    if before.status.id != after.status.id {
        activity_log::repo::record_status_change(&state.db, ticket_id, user_id, before.status.id, after.status.id, access_log_id).await?;

        // Just reached "done" (completed, but not yet confirmed) - ask a
        // project manager to review and confirm/close it. Not fired when the
        // transition lands directly on "confirmed" (nothing to request) or
        // when the ticket was already done/confirmed before this update.
        if after.status.category == StatusCategory::Done && before.status.category != StatusCategory::Done {
            crate::notifications::repo::notify_confirmation_requested(&state.db, after.project_id, user_id, ticket_id).await?;
        }
    }

    if before.assignee_id != after.assignee_id {
        activity_log::repo::record_assignee_change(&state.db, ticket_id, user_id, before.assignee_id, after.assignee_id, access_log_id).await?;
        if let Some(assignee_id) = after.assignee_id {
            crate::notifications::repo::notify_ticket_assigned(&state.db, assignee_id, user_id, ticket_id).await?;
        }
    }

    if before.title != after.title {
        activity_log::repo::record_field_change(
            &state.db, ticket_id, user_id, "title", Some(before.title.clone()), Some(after.title.clone()), access_log_id,
        ).await?;
    }
    if before.description != after.description {
        activity_log::repo::record_field_change(
            &state.db,
            ticket_id,
            user_id,
            "description",
            before.description.as_deref().map(truncate_text),
            after.description.as_deref().map(truncate_text),
            access_log_id,
        ).await?;

        let old_text = before.description.as_deref().unwrap_or("");
        let new_text = after.description.as_deref().unwrap_or("");
        for mentioned_id in crate::mentions::newly_mentioned(old_text, new_text) {
            crate::notifications::repo::notify_mentioned_in_ticket(&state.db, mentioned_id, user_id, ticket_id).await?;
        }
    }
    if before.due_date != after.due_date {
        activity_log::repo::record_field_change(
            &state.db, ticket_id, user_id, "due_date",
            before.due_date.map(|d| d.to_string()), after.due_date.map(|d| d.to_string()), access_log_id,
        ).await?;
    }
    if before.start_date != after.start_date {
        activity_log::repo::record_field_change(
            &state.db, ticket_id, user_id, "start_date",
            before.start_date.map(|d| d.to_string()), after.start_date.map(|d| d.to_string()), access_log_id,
        ).await?;
    }
    if before.end_date != after.end_date {
        activity_log::repo::record_field_change(
            &state.db, ticket_id, user_id, "end_date",
            before.end_date.map(|d| d.to_string()), after.end_date.map(|d| d.to_string()), access_log_id,
        ).await?;
    }
    if before.milestone_id != after.milestone_id {
        let from_text = milestone_text(state, before.milestone_id).await?;
        let to_text = milestone_text(state, after.milestone_id).await?;
        activity_log::repo::record_field_change(&state.db, ticket_id, user_id, "milestone_id", from_text, to_text, access_log_id).await?;
    }
    if before.parent_ticket_id != after.parent_ticket_id {
        let from_text = parent_ticket_text(state, before.parent_ticket_id).await?;
        let to_text = parent_ticket_text(state, after.parent_ticket_id).await?;
        activity_log::repo::record_field_change(&state.db, ticket_id, user_id, "parent_ticket_id", from_text, to_text, access_log_id).await?;
    }

    Ok(())
}

pub async fn delete_ticket(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    let ticket = repo::get_ticket(&state.db, id).await?.ok_or(AppError::NotFound)?;
    access::check_project_access(&state.db, &user, ticket.project_id, Access::Write).await?;
    repo::delete_ticket(&state.db, id).await?;
    crate::audit::repo::record(&state.db, user.id, "ticket_deleted", "ticket", Some(id), Some(&ticket.title), None, ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_ticket_activity(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
    Query(page): Query<PageQuery>,
) -> Result<Json<Paginated<activity_log::TicketActivity>>> {
    let project_id = access::project_id_for_ticket(&state.db, id).await?;
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let activities = activity_log::repo::list_for_ticket(&state.db, id, page.limit(), page.offset()).await?;
    let total = activity_log::repo::count_for_ticket(&state.db, id).await?;
    Ok(Json(Paginated::new(activities, total, &page)))
}

#[derive(serde::Serialize)]
pub struct ActualDurationResponse {
    // How long the ticket has (so far) spent in an in_progress-category
    // status - see tickets::duration::in_progress_seconds for the exact
    // reconstruction from its status_change history.
    pub in_progress_seconds: i64,
}

pub async fn get_actual_duration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<ActualDurationResponse>> {
    let ticket = repo::get_ticket(&state.db, id).await?.ok_or(AppError::NotFound)?;
    access::check_project_access(&state.db, &user, ticket.project_id, Access::Read).await?;

    let changes = activity_log::repo::list_status_changes_for_ticket(&state.db, id).await?;
    let statuses = crate::workflows::repo::list_statuses(&state.db, ticket.status.workflow_id).await?;
    let categories: std::collections::HashMap<i64, StatusCategory> =
        statuses.into_iter().map(|s| (s.id, s.category)).collect();
    let changes: Vec<super::duration::StatusChange> = changes
        .into_iter()
        .map(|(occurred_at, from_status_id, to_status_id)| super::duration::StatusChange {
            occurred_at,
            from_status_id,
            to_status_id,
        })
        .collect();

    let in_progress_seconds = super::duration::in_progress_seconds(
        ticket.created_at,
        ticket.status.id,
        &changes,
        &categories,
        chrono::Utc::now(),
    );

    Ok(Json(ActualDurationResponse { in_progress_seconds }))
}
