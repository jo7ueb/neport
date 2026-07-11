use axum::{extract::{Path, State}, Json};
use std::sync::Arc;

use crate::{access::{self, Access}, auth::extractors::AuthUser, error::Result, state::AppState};
use super::{models::GanttData, repo};

pub async fn get_gantt(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(project_id): Path<i64>,
) -> Result<Json<GanttData>> {
    access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
    let data = repo::get_gantt_data(&state.db, project_id).await?;
    Ok(Json(data))
}
