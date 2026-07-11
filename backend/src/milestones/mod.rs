pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::get, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_milestones).post(handlers::create_milestone))
        .route("/:id", get(handlers::get_milestone).patch(handlers::update_milestone).delete(handlers::delete_milestone))
        .route("/:id/burndown", get(handlers::get_burndown))
}
