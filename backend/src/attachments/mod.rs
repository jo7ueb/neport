pub mod handlers;
pub mod repo;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use crate::state::AppState;

pub const MAX_ATTACHMENT_BYTES: usize = 8 * 1024 * 1024;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::upload_attachment))
        .route("/:filename", get(handlers::get_attachment))
        .layer(DefaultBodyLimit::max(MAX_ATTACHMENT_BYTES))
}
