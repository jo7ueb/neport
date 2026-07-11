use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSession {
    pub id: i64,
    pub user_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayStatus {
    InProgress,
    Valid,
    Invalid,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: NaiveDate,
    pub sessions: Vec<WorkSession>,
    pub first_started_at: DateTime<Utc>,
    pub last_ended_at: Option<DateTime<Utc>>,
    pub status: DayStatus,
    // Set only when status is Approved/Rejected - the current, non-stale
    // approval decision (see approvals::repo::get_current_decision).
    pub decision: Option<crate::approvals::models::DecisionRecord>,
}
