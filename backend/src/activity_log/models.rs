use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Comment,
    StatusChange,
    AssigneeChange,
    FieldUpdate,
    Created,
}

impl FromStr for ActivityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "comment" => Ok(ActivityType::Comment),
            "status_change" => Ok(ActivityType::StatusChange),
            "assignee_change" => Ok(ActivityType::AssigneeChange),
            "field_update" => Ok(ActivityType::FieldUpdate),
            "created" => Ok(ActivityType::Created),
            _ => Err(format!("invalid activity type: {}", s)),
        }
    }
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ActivityType::Comment => "comment",
            ActivityType::StatusChange => "status_change",
            ActivityType::AssigneeChange => "assignee_change",
            ActivityType::FieldUpdate => "field_update",
            ActivityType::Created => "created",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketActivity {
    pub id: i64,
    pub ticket_id: i64,
    pub user_id: i64,
    pub activity_type: ActivityType,
    pub occurred_at: DateTime<Utc>,
    // Only populated for status_change / assignee_change rows respectively
    // (null for every other type, and for rows logged before this column
    // existed). The frontend already has workflow statuses and users
    // loaded, so it resolves these ids to labels itself.
    pub from_status_id: Option<i64>,
    pub to_status_id: Option<i64>,
    pub from_assignee_id: Option<i64>,
    pub to_assignee_id: Option<i64>,
    // Only populated for field_update rows. field_name identifies which
    // single field changed ("title", "description", "due_date", ...);
    // from_text/to_text are a plain snapshot of that field's value (null
    // means the field was empty, not that data is missing - field_name
    // being null is what marks a legacy row with no diff at all).
    pub field_name: Option<String>,
    pub from_text: Option<String>,
    pub to_text: Option<String>,
}
