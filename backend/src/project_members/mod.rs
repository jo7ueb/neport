pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::get, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_members).post(handlers::add_member))
        .route(
            "/:user_id",
            axum::routing::patch(handlers::update_member).delete(handlers::remove_member),
        )
}
