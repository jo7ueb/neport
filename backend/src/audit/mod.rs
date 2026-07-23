pub mod client_ip;
pub mod handlers;
pub mod models;
pub mod repo;

use axum::{routing::get, Router};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(handlers::list_audit_log))
}
