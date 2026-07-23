use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{access::{self, Access}, auth::extractors::AuthUser, auth::UserRole, error::{AppError, Result}, pagination::{PageQuery, Paginated}, state::AppState};
use crate::project_members::models::ProjectRole;
use crate::workflows::{models::WorkflowDetail, repo as workflows_repo};
use super::{models::Project, repo};

#[derive(Debug, Deserialize)]
pub struct MemberInput {
    pub user_id: i64,
    pub role: ProjectRole,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: Option<i64>,
    pub members: Option<Vec<MemberInput>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListProjectsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    // "all" lets a site admin opt into seeing every project; any other value
    // (or omission) defaults to "only projects I'm involved in", same as a
    // general user, so an admin's own project list isn't cluttered with
    // projects they have no stake in.
    pub scope: Option<String>,
    // The "アーカイブ" tab in the project list - a separate view, not merged
    // into the normal list (see projects::repo::list_projects).
    #[serde(default)]
    pub archived: bool,
}

pub async fn list_projects(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<Paginated<Project>>> {
    let is_admin = user.role == UserRole::Admin;
    let show_all = is_admin && query.scope.as_deref() == Some("all");
    let page = PageQuery { page: query.page, per_page: query.per_page };
    let projects = repo::list_projects(&state.db, query.archived, user.id, show_all, page.limit(), page.offset()).await?;
    let total = repo::count_projects(&state.db, query.archived, user.id, show_all).await?;
    Ok(Json(Paginated::new(projects, total, &page)))
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    crate::auth::extractors::AdminUser(user): crate::auth::extractors::AdminUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>)> {
    let members = payload.members.unwrap_or_default();
    if !members.is_empty() && !members.iter().any(|m| m.role == ProjectRole::Manager) {
        return Err(AppError::BadRequest("a project must have at least one manager".to_string()));
    }
    let initial_members: Vec<(i64, ProjectRole)> = members.into_iter().map(|m| (m.user_id, m.role)).collect();
    let project = repo::create_project(
        &state.db,
        &payload.name,
        payload.description.as_deref(),
        payload.workflow_id,
        user.id,
        &initial_members,
    )
    .await?;
    crate::audit::repo::record(&state.db, user.id, "project_created", "project", Some(project.id), None, None, ip.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn get_project_workflow(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<WorkflowDetail>> {
    access::check_project_access(&state.db, &user, id, Access::Read).await?;
    let project = repo::get_project(&state.db, id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;
    let detail = workflows_repo::get_workflow_detail(&state.db, project.workflow_id)
        .await?
        .ok_or(crate::error::AppError::InternalServerError)?;
    Ok(Json(detail))
}

pub async fn get_project(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Project>> {
    access::check_project_access(&state.db, &user, id, Access::Read).await?;
    let project = repo::get_project(&state.db, id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;
    Ok(Json(project))
}

pub async fn update_project(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<Json<Project>> {
    access::check_project_access(&state.db, &user, id, Access::Manage).await?;
    let project = repo::update_project(&state.db, id, payload.name.as_deref(), payload.description.as_deref())
        .await?
        .ok_or(crate::error::AppError::NotFound)?;
    crate::audit::repo::record(&state.db, user.id, "project_updated", "project", Some(id), None, None, ip.as_deref()).await?;
    Ok(Json(project))
}

// Archiving/unarchiving is a site-admin-only action (not delegated to
// project managers like Access::Manage is), so this goes straight to
// AdminUser rather than through check_project_access.
pub async fn archive_project(
    State(state): State<Arc<AppState>>,
    crate::auth::extractors::AdminUser(user): crate::auth::extractors::AdminUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    repo::get_project(&state.db, id).await?.ok_or(AppError::NotFound)?;
    repo::archive_project(&state.db, id).await?;
    crate::audit::repo::record(&state.db, user.id, "project_archived", "project", Some(id), None, None, ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unarchive_project(
    State(state): State<Arc<AppState>>,
    crate::auth::extractors::AdminUser(user): crate::auth::extractors::AdminUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    repo::get_project(&state.db, id).await?.ok_or(AppError::NotFound)?;
    repo::unarchive_project(&state.db, id).await?;
    crate::audit::repo::record(&state.db, user.id, "project_unarchived", "project", Some(id), None, None, ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}
