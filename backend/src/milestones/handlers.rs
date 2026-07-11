use axum::{extract::{Path, State}, http::StatusCode, Json};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;

use crate::{access::{self, Access}, auth::extractors::AuthUser, error::{AppError, Result}, state::AppState};
use super::{models::{BurndownData, Milestone}, repo};

#[derive(Debug, Deserialize)]
pub struct CreateMilestoneRequest {
    pub name: String,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMilestoneRequest {
    pub name: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub description: Option<String>,
}

pub async fn list_milestones(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(project_id): Path<i64>,
) -> Result<Json<Vec<Milestone>>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let milestones = repo::list_milestones_for_project(&state.db, project_id).await?;
    Ok(Json(milestones))
}

pub async fn create_milestone(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(project_id): Path<i64>,
    Json(payload): Json<CreateMilestoneRequest>,
) -> Result<(StatusCode, Json<Milestone>)> {
    access::check_project_access(&state.db, &user, project_id, Access::Write).await?;
    let milestone = repo::create_milestone(
        &state.db,
        project_id,
        &payload.name,
        payload.start_date,
        payload.due_date,
        payload.description.as_deref(),
        Some(user.id),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(milestone)))
}

pub async fn get_milestone(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path((project_id, id)): Path<(i64, i64)>,
) -> Result<Json<Milestone>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let milestone = repo::get_milestone(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(milestone))
}

pub async fn update_milestone(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path((project_id, id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateMilestoneRequest>,
) -> Result<Json<Milestone>> {
    access::check_project_access(&state.db, &user, project_id, Access::Write).await?;
    let milestone = repo::update_milestone(
        &state.db,
        id,
        payload.name.as_deref(),
        payload.start_date,
        payload.due_date,
        payload.description.as_deref(),
    )
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(milestone))
}

pub async fn get_burndown(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path((project_id, id)): Path<(i64, i64)>,
) -> Result<Json<BurndownData>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let data = repo::burndown_data(&state.db, id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(data))
}

pub async fn delete_milestone(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path((project_id, id)): Path<(i64, i64)>,
) -> Result<StatusCode> {
    access::check_project_access(&state.db, &user, project_id, Access::Write).await?;
    repo::delete_milestone(&state.db, id, Some(user.id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
