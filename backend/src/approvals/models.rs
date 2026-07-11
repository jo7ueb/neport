use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Approved,
    Rejected,
}

impl std::str::FromStr for Decision {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "approved" => Ok(Decision::Approved),
            "rejected" => Ok(Decision::Rejected),
            _ => Err(format!("Invalid decision: {}", s)),
        }
    }
}

impl ToString for Decision {
    fn to_string(&self) -> String {
        match self {
            Decision::Approved => "approved".to_string(),
            Decision::Rejected => "rejected".to_string(),
        }
    }
}

// One row of the append-only daily_approvals log. Never mutated - see the
// migration comment for how "the current decision" is derived from these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision: Decision,
    pub comment: String,
    pub approver_id: i64,
    pub approver_name: String,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockReason {
    Approved,
    MonthClosed,
}

// Derived from the latest row in monthly_closures for a (user, year, month).
#[derive(Debug, Clone, Serialize)]
pub struct ClosureState {
    pub closed: bool,
    pub closed_by: Option<i64>,
    pub closed_by_name: Option<String>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct DecideRequest {
    pub decision: Decision,
    #[serde(default)]
    pub comment: String,
}
