use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::extractors::AuthUser, error::Result, pagination::{PageQuery, Paginated}, state::AppState};
use super::{models::Notification, repo};

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub unread_only: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<Paginated<Notification>>> {
    let page = PageQuery { page: query.page, per_page: query.per_page };
    let unread_only = query.unread_only.unwrap_or(false);
    let notifications = repo::list_for_user(&state.db, user.id, unread_only, page.limit(), page.offset()).await?;
    let total = repo::count_for_user(&state.db, user.id, unread_only).await?;
    Ok(Json(Paginated::new(notifications, total, &page)))
}

#[derive(serde::Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

pub async fn unread_count(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<UnreadCountResponse>> {
    let count = repo::count_for_user(&state.db, user.id, true).await?;
    Ok(Json(UnreadCountResponse { count }))
}

pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    repo::mark_read(&state.db, user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_all_read(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<StatusCode> {
    repo::mark_all_read(&state.db, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
