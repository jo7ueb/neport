use axum::{async_trait, extract::FromRequestParts, http::{request::Parts, Method}};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;
use super::{session, models::{User, UserRole, UserStatus}};

pub struct AuthUser(pub User);

/// Cookie/session/CSRF/status checks shared by every extractor below.
/// Deliberately does *not* apply the maintenance-mode gate, since that gate
/// must not apply uniformly - see `AuthUser` vs `MaintenanceModeAdmin`.
async fn authenticate(parts: &mut Parts, state: &Arc<AppState>) -> Result<User, AppError> {
    let write = !matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS);
    if let Some(value) = parts.headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = value.strip_prefix("Bearer ") {
            let user_id = crate::auth::api_tokens::authenticate(&state.db, token.trim(), write).await?;
            return crate::users::repo::get_user_by_id(&state.db, user_id)
                .await?.filter(|u| u.status == UserStatus::Active).ok_or(AppError::Unauthorized);
        }
        return Err(AppError::Unauthorized);
    }
    let cookie_header = parts
        .headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let session_id = parse_cookie(cookie_header, &state.session_cookie_name)
        .and_then(|v| Uuid::parse_str(&v).ok())
        .ok_or(AppError::Unauthorized)?;

    let sess = session::get_session(&state.db, session_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if write {
        let provided = parts
            .headers
            .get("X-CSRF-Token")
            .and_then(|v| v.to_str().ok());
        if provided != Some(sess.csrf_token.as_str()) {
            return Err(AppError::InvalidCsrf);
        }
    }

    let user = crate::users::repo::get_user_by_id(&state.db, sess.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.status != UserStatus::Active {
        return Err(AppError::Forbidden);
    }

    Ok(user)
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, AppError> {
        let user = authenticate(parts, state).await?;

        // Maintenance mode pauses ALL writes (including admins') so nothing
        // races a backup/restore snapshot. Reads are never blocked. The only
        // exemption is `MaintenanceModeAdmin`, used solely by the handful of
        // endpoints that must keep working *during* maintenance mode:
        // toggling the flag itself back off, and creating/restoring backups.
        if !matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS)
            && crate::settings::repo::is_maintenance_mode(&state.db).await?
        {
            return Err(AppError::MaintenanceMode);
        }

        Ok(AuthUser(user))
    }
}

pub struct AdminUser(pub User);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, AppError> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err(AppError::Forbidden);
        }
        Ok(AdminUser(user))
    }
}

/// Like `AdminUser`, but does not apply the maintenance-mode gate. Reserved
/// for the endpoints that must remain callable *while* maintenance mode is
/// on: disabling the flag itself (`PATCH /settings`) and backup create/restore
/// (restore in particular is only even allowed while the flag is on).
pub struct MaintenanceModeAdmin(pub User);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for MaintenanceModeAdmin {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, AppError> {
        let user = authenticate(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err(AppError::Forbidden);
        }
        Ok(MaintenanceModeAdmin(user))
    }
}

pub(crate) fn parse_cookie(header: &str, name: &str) -> Option<String> {
    header.split(';').find_map(|pair| {
        let mut parts = pair.trim().splitn(2, '=');
        let k = parts.next()?;
        let v = parts.next()?;
        (k == name).then(|| v.to_string())
    })
}

/// Extracts the current session id from the request's Cookie header, for
/// handlers (auth_me, auth_logout, change_password) that need the raw
/// session id itself rather than the resolved User the AuthUser extractor
/// provides.
pub fn session_id_from_headers(headers: &axum::http::HeaderMap, cookie_name: &str) -> Option<Uuid> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| parse_cookie(h, cookie_name))
        .and_then(|v| Uuid::parse_str(&v).ok())
}
