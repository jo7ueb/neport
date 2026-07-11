use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub katex_enabled: bool,
    // IANA timezone name (e.g. "Asia/Tokyo") used for any user who hasn't
    // set their own timezone override.
    pub default_timezone: String,
    // While true, non-admin write requests are rejected (503) - see
    // auth::extractors::AuthUser. Meant to be flipped on briefly around a
    // backup/restore so no writes race the snapshot.
    pub maintenance_mode: bool,
}
