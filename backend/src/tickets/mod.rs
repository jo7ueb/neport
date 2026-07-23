pub mod comments;
pub mod dependencies;
pub mod duration;
pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::{delete, get, post}, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::create_ticket))
        .route("/search", get(handlers::search_tickets))
        .route("/:id", get(handlers::get_ticket).patch(handlers::update_ticket).delete(handlers::delete_ticket))
        .route("/:id/children", get(handlers::list_children))
        .route("/:id/dependencies", get(dependencies::handlers::list_dependencies).post(dependencies::handlers::create_dependency))
        .route("/:id/dependencies/:dependency_id", delete(dependencies::handlers::delete_dependency))
        .route("/:id/comments", get(comments::handlers::list_comments).post(comments::handlers::create_comment))
        .route("/:id/activity", get(handlers::list_ticket_activity))
        .route("/:id/actual-duration", get(handlers::get_actual_duration))
}
