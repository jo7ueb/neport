use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    access::{self, Access},
    auth::extractors::AuthUser,
    error::{AppError, Result},
    state::AppState,
};
use super::{
    models::{ProjectMember, ProjectRole},
    repo,
};

pub async fn list_members(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(project_id): Path<i64>,
) -> Result<Json<Vec<ProjectMember>>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let members = repo::list_members(&state.db, project_id).await?;
    Ok(Json(members))
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: i64,
    pub role: ProjectRole,
}

pub async fn add_member(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path(project_id): Path<i64>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<(StatusCode, Json<ProjectMember>)> {
    access::check_project_access(&state.db, &user, project_id, Access::Manage).await?;
    let member = repo::add_member(&state.db, project_id, payload.user_id, payload.role, Some(user.id)).await?;
    crate::audit::repo::record(
        &state.db,
        user.id,
        "project_member_added",
        "project_member",
        Some(payload.user_id),
        None,
        Some(&format!("project_id={}, role={}", project_id, payload.role)),
        ip.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(member)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRequest {
    pub role: ProjectRole,
}

pub async fn update_member(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path((project_id, member_user_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateMemberRequest>,
) -> Result<StatusCode> {
    access::check_project_access(&state.db, &user, project_id, Access::Manage).await?;

    if payload.role != ProjectRole::Manager {
        guard_not_last_manager(&state, project_id, member_user_id).await?;
    }

    let previous_role = repo::get_role(&state.db, project_id, member_user_id).await?;
    repo::update_member_role(&state.db, project_id, member_user_id, payload.role).await?;
    crate::audit::repo::record(
        &state.db,
        user.id,
        "project_member_role_changed",
        "project_member",
        Some(member_user_id),
        previous_role.map(|r| r.to_string()).as_deref(),
        Some(&format!("project_id={}, role={}", project_id, payload.role)),
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Path((project_id, member_user_id)): Path<(i64, i64)>,
) -> Result<StatusCode> {
    access::check_project_access(&state.db, &user, project_id, Access::Manage).await?;

    guard_not_last_manager(&state, project_id, member_user_id).await?;

    repo::remove_member(&state.db, project_id, member_user_id).await?;
    crate::audit::repo::record(
        &state.db,
        user.id,
        "project_member_removed",
        "project_member",
        Some(member_user_id),
        None,
        Some(&project_id.to_string()),
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// A project must always keep at least one manager, or nobody (short of a
// site admin) would ever be able to manage it again.
async fn guard_not_last_manager(state: &AppState, project_id: i64, member_user_id: i64) -> Result<()> {
    let current_role = repo::get_role(&state.db, project_id, member_user_id).await?;
    if current_role == Some(ProjectRole::Manager) {
        let manager_count = repo::count_managers(&state.db, project_id).await?;
        if manager_count <= 1 {
            return Err(AppError::BadRequest(
                "a project must have at least one manager".to_string(),
            ));
        }
    }
    Ok(())
}
