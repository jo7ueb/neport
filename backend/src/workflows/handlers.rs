use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    auth::extractors::{AdminUser, AuthUser},
    error::{AppError, Result},
    state::AppState,
};
use super::{
    models::{StatusCategory, Workflow, WorkflowDetail, WorkflowStatus, WorkflowTransition},
    repo,
};

pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
) -> Result<Json<Vec<Workflow>>> {
    let workflows = repo::list_workflows(&state.db).await?;
    Ok(Json(workflows))
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
}

pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<Workflow>)> {
    let workflow = repo::create_workflow(&state.db, &payload.name, Some(admin.id)).await?;
    Ok((StatusCode::CREATED, Json(workflow)))
}

pub async fn get_workflow_detail(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<WorkflowDetail>> {
    let detail = repo::get_workflow_detail(&state.db, id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(detail))
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: String,
}

pub async fn update_workflow(
    State(state): State<Arc<AppState>>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateWorkflowRequest>,
) -> Result<Json<Workflow>> {
    let workflow = repo::update_workflow(&state.db, id, &payload.name).await?.ok_or(AppError::NotFound)?;
    Ok(Json(workflow))
}

pub async fn delete_workflow(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    let workflow = repo::get_workflow(&state.db, id).await?.ok_or(AppError::NotFound)?;
    if workflow.is_default {
        return Err(AppError::BadRequest("the default workflow cannot be deleted".to_string()));
    }
    repo::delete_workflow(&state.db, id).await?;
    crate::audit::repo::record(&state.db, admin.id, "workflow_deleted", "workflow", Some(id), Some(&workflow.name), None).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct CreateStatusRequest {
    pub key: String,
    pub label: String,
    pub category: StatusCategory,
    pub position: i32,
    #[serde(default)]
    pub is_initial: bool,
}

pub async fn create_status(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path(workflow_id): Path<i64>,
    Json(payload): Json<CreateStatusRequest>,
) -> Result<(StatusCode, Json<WorkflowStatus>)> {
    let status = repo::create_status(
        &state.db,
        workflow_id,
        &payload.key,
        &payload.label,
        payload.category,
        payload.position,
        payload.is_initial,
        Some(admin.id),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(status)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub key: Option<String>,
    pub label: Option<String>,
    pub category: Option<StatusCategory>,
    pub position: Option<i32>,
    pub is_initial: Option<bool>,
}

pub async fn update_status(
    State(state): State<Arc<AppState>>,
    AdminUser(_admin): AdminUser,
    Path((_workflow_id, status_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<Json<WorkflowStatus>> {
    let status = repo::update_status(
        &state.db,
        status_id,
        payload.key.as_deref(),
        payload.label.as_deref(),
        payload.category,
        payload.position,
        payload.is_initial,
    )
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(status))
}

pub async fn delete_status(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path((workflow_id, status_id)): Path<(i64, i64)>,
) -> Result<StatusCode> {
    let label = repo::list_statuses(&state.db, workflow_id)
        .await?
        .into_iter()
        .find(|s| s.id == status_id)
        .map(|s| s.label);
    repo::delete_status(&state.db, status_id).await?;
    crate::audit::repo::record(&state.db, admin.id, "workflow_status_deleted", "workflow_status", Some(status_id), label.as_deref(), None).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct CreateTransitionRequest {
    pub from_status_id: i64,
    pub to_status_id: i64,
}

pub async fn create_transition(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path(workflow_id): Path<i64>,
    Json(payload): Json<CreateTransitionRequest>,
) -> Result<(StatusCode, Json<WorkflowTransition>)> {
    let transition = repo::create_transition(&state.db, workflow_id, payload.from_status_id, payload.to_status_id, Some(admin.id)).await?;
    Ok((StatusCode::CREATED, Json(transition)))
}

pub async fn delete_transition(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path((workflow_id, transition_id)): Path<(i64, i64)>,
) -> Result<StatusCode> {
    let label = repo::list_transitions(&state.db, workflow_id)
        .await?
        .into_iter()
        .find(|t| t.id == transition_id)
        .map(|t| format!("{} -> {}", t.from_status_id, t.to_status_id));
    repo::delete_transition(&state.db, transition_id).await?;
    crate::audit::repo::record(&state.db, admin.id, "workflow_transition_deleted", "workflow_transition", Some(transition_id), label.as_deref(), None).await?;
    Ok(StatusCode::NO_CONTENT)
}
