pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_projects).post(handlers::create_project))
        .route("/:id", get(handlers::get_project).patch(handlers::update_project))
        .route("/:id/tickets/summary", get(handlers::get_ticket_summary))
        .route("/:id/archive", post(handlers::archive_project))
        .route("/:id/unarchive", post(handlers::unarchive_project))
        .route("/:id/workflow", get(handlers::get_project_workflow))
        .route("/:id/tickets", get(crate::tickets::handlers::list_tickets))
        .route("/:id/tickets/tree", get(crate::tickets::handlers::get_ticket_tree))
}
