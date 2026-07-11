pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_notifications))
        .route("/unread-count", get(handlers::unread_count))
        .route("/read-all", post(handlers::mark_all_read))
        .route("/:id/read", post(handlers::mark_read))
}
