pub mod handlers;
pub mod service;

use axum::{extract::DefaultBodyLimit, routing::post, Router};
use std::sync::Arc;
use crate::state::AppState;

// A full backup bundle (db dump + every attachment) can be much larger than
// a single attachment upload's 8MB cap (attachments::MAX_ATTACHMENT_BYTES) -
// this covers both routes since create_backup's request has no body anyway.
const MAX_BACKUP_BYTES: usize = 500 * 1024 * 1024;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::create_backup))
        .route("/restore", post(handlers::restore_backup))
        .layer(DefaultBodyLimit::max(MAX_BACKUP_BYTES))
}
