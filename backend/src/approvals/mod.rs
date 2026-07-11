pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/subordinates", get(handlers::list_subordinates))
        .route("/subordinates/:user_id/monthly", get(handlers::monthly))
        .route("/subordinates/:user_id/day/:date", get(handlers::get_day))
        .route("/subordinates/:user_id/day/:date/decide", post(handlers::decide))
        .route("/subordinates/:user_id/month/:year/:month/close", post(handlers::close_month))
        .route("/subordinates/:user_id/month/:year/:month/reopen", post(handlers::reopen_month))
}
