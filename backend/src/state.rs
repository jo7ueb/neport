use sqlx::PgPool;
use std::sync::Arc;

use crate::storage::AttachmentStorage;

pub struct AppState {
    pub db: PgPool,
    pub session_cookie_name: String,
    pub session_ttl_seconds: u64,
    // Local disk (docker-compose) or GCS (Cloud Run) - see backend/src/storage.
    pub storage: Arc<dyn AttachmentStorage>,
    // Kept alongside the connected `db` pool so backend/src/backup can shell
    // out to pg_dump/pg_restore with the exact same connection info the app
    // itself uses - see backup::service.
    pub database_url: String,
}
