pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::get, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_reports).post(handlers::submit_report))
        .route("/today/tickets", get(handlers::get_today_tickets))
        .route("/:date", get(handlers::get_report))
}
