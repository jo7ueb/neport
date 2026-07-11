pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::{get, patch, post}, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/start", post(handlers::start))
        .route("/end", post(handlers::end))
        .route("/current", get(handlers::current))
        .route("/today", get(handlers::today))
        .route("/day", get(handlers::day))
        .route("/monthly", get(handlers::monthly))
        .route("/:id", patch(handlers::update))
}
