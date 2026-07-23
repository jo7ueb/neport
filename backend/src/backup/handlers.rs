use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;

use crate::{auth::extractors::MaintenanceModeAdmin, error::{AppError, Result}, state::AppState};
use super::service;

pub async fn create_backup(
    State(state): State<Arc<AppState>>,
    MaintenanceModeAdmin(admin): MaintenanceModeAdmin,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
) -> Result<impl IntoResponse> {
    let temp_dir = service::TempDir::new().await?;
    let dump_path = temp_dir.path().join("db.dump");

    service::run_pg_dump(&state.database_url, &dump_path).await?;

    // Read every attachment into memory up front (storage I/O may be
    // network-bound for GCS) - the actual tar/gzip building below stays
    // synchronous and storage-agnostic, see backup::service.
    let mut uploads = Vec::new();
    for name in state.storage.list().await? {
        if let Some(data) = state.storage.read(&name).await? {
            uploads.push((name, data));
        }
    }

    let bytes = {
        let dump_path = dump_path.clone();
        tokio::task::spawn_blocking(move || service::build_backup_archive(&dump_path, &uploads))
            .await
            .map_err(|_| AppError::InternalServerError)??
    };

    let filename = format!("neport-backup-{}.tar.gz", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    crate::audit::repo::record(&state.db, admin.id, "backup_created", "backup", None, None, Some(&filename), ip.as_deref()).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/gzip".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    ))
}

pub async fn restore_backup(
    State(state): State<Arc<AppState>>,
    MaintenanceModeAdmin(admin): MaintenanceModeAdmin,
    crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
    mut multipart: Multipart,
) -> Result<StatusCode> {
    // The upload body must always be fully read before this handler can
    // return, successfully or not - responding early while the client is
    // still streaming a large multipart body leaves it undrained, which can
    // poison the HTTP keep-alive connection for whatever request reuses it
    // next. So the maintenance-mode precondition is checked *after* reading,
    // not before.
    let mut file_data: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("invalid upload".into()))?
    {
        if field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("failed to read uploaded file".into()))?;
            file_data = Some(data.to_vec());
        }
    }
    let data = file_data.ok_or_else(|| AppError::BadRequest("no file uploaded".into()))?;

    // Requiring maintenance mode forces a deliberate two-step action (enable
    // it, then upload) and guarantees no other write races the restore. This
    // check only enforces the *precondition* for restore specifically; the
    // general "maintenance mode blocks writes" rule is enforced separately
    // in auth::extractors::AuthUser (this handler is exempt from it via
    // MaintenanceModeAdmin, since it must be callable while the flag is on).
    if !crate::settings::repo::is_maintenance_mode(&state.db).await? {
        return Err(AppError::BadRequest("enable maintenance mode before restoring a backup".to_string()));
    }

    let temp_dir = service::TempDir::new().await?;
    let dump_path = temp_dir.path().join("db.dump");

    let uploads = {
        let dump_path = dump_path.clone();
        tokio::task::spawn_blocking(move || service::extract_backup_archive(&data, &dump_path))
            .await
            .map_err(|_| AppError::InternalServerError)??
    };

    // Replace the entire attachment set to match the backup exactly (files
    // removed since the backup was taken shouldn't linger).
    state.storage.clear().await?;
    for (name, bytes) in uploads {
        state.storage.write(&name, &bytes).await?;
    }

    service::run_pg_restore(&state.database_url, &dump_path).await?;

    // Recorded after, not before: pg_restore --clean replaces audit_log's
    // own contents with whatever the backup had, so a record written
    // beforehand would just be wiped out by the restore it's describing.
    crate::audit::repo::record(&state.db, admin.id, "backup_restored", "backup", None, None, None, ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}
