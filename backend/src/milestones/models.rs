use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by_id: Option<i64>,
    pub total_story_points: i64,
    pub done_story_points: i64,
    pub remaining_story_points: i64,
    pub remaining_estimated_hours: f64,
    pub unestimated_ticket_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BurndownPoint {
    pub date: NaiveDate,
    pub remaining: i64,
    pub ideal: Option<f64>,
}

// start_date is optional here (unlike Milestone.start_date being required to
// compute anything below) so the endpoint can return a normal 200 with empty
// points when a milestone hasn't had its start_date set yet, rather than
// forcing the frontend to distinguish that case from a real 404.
#[derive(Debug, Clone, Serialize)]
pub struct BurndownData {
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub total_at_start: i64,
    pub unestimated_ticket_count: i64,
    pub points: Vec<BurndownPoint>,
}
