pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_workflows).post(handlers::create_workflow))
        .route("/:id", get(handlers::get_workflow_detail).patch(handlers::update_workflow).delete(handlers::delete_workflow))
        .route("/:id/statuses", post(handlers::create_status))
        .route("/:id/statuses/:status_id", axum::routing::patch(handlers::update_status).delete(handlers::delete_status))
        .route("/:id/transitions", post(handlers::create_transition))
        .route("/:id/transitions/:transition_id", axum::routing::delete(handlers::delete_transition))
}
