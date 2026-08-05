use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: i64,
    pub hours_per_story_point: f64,
    pub created_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_by_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTicketSummary {
    pub remaining_story_points: i64,
    pub remaining_estimated_hours: f64,
    pub unestimated_ticket_count: i64,
}
