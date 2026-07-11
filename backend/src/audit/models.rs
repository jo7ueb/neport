use chrono::{DateTime, Utc};
use serde::Serialize;

// actor_name/target_name are resolved via JOIN at read time (not
// snapshotted), same tradeoff as notifications::Notification - a renamed
// user always shows their current display name here.
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub actor_id: i64,
    pub actor_name: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<i64>,
    pub target_name: Option<String>,
    pub from_text: Option<String>,
    pub to_text: Option<String>,
    pub occurred_at: DateTime<Utc>,
}
