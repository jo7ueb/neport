use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    auth::extractors::{AuthUser, MaintenanceModeAdmin},
    error::Result,
    state::AppState,
};
use super::{models::SystemSettings, repo};

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
) -> Result<Json<SystemSettings>> {
    let settings = repo::get_settings(&state.db).await?;
    Ok(Json(settings))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub katex_enabled: bool,
    pub default_timezone: String,
    pub maintenance_mode: bool,
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    MaintenanceModeAdmin(admin): MaintenanceModeAdmin,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<SystemSettings>> {
    if !crate::time::is_valid_timezone(&payload.default_timezone) {
        return Err(crate::error::AppError::BadRequest("invalid timezone".to_string()));
    }
    let before = repo::get_settings(&state.db).await?;
    let settings = repo::update_settings(&state.db, payload.katex_enabled, &payload.default_timezone, payload.maintenance_mode).await?;
    crate::audit::repo::record(
        &state.db,
        admin.id,
        "system_settings_updated",
        "system_settings",
        None,
        Some(&format!(
            "katex_enabled={}, default_timezone={}, maintenance_mode={}",
            before.katex_enabled, before.default_timezone, before.maintenance_mode
        )),
        Some(&format!(
            "katex_enabled={}, default_timezone={}, maintenance_mode={}",
            settings.katex_enabled, settings.default_timezone, settings.maintenance_mode
        )),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(settings))
}
