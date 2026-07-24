use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusCategory {
    Todo,
    InProgress,
    // The assignee marked the ticket complete, but a project manager hasn't
    // reviewed it yet - see StatusCategory::Confirmed.
    Done,
    // A project manager reviewed a "done" ticket and closed it. Distinct
    // from Done because completion, as claimed by whoever did the work, and
    // confirmation, as verified by whoever manages the project, are not the
    // same event - only Confirmed counts as "really finished" for Gantt
    // progress and only a project manager (or site admin) may transition a
    // ticket into a Confirmed-category status (see access::Access::Manage).
    Confirmed,
}

impl FromStr for StatusCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(StatusCategory::Todo),
            "in_progress" => Ok(StatusCategory::InProgress),
            "done" => Ok(StatusCategory::Done),
            "confirmed" => Ok(StatusCategory::Confirmed),
            _ => Err(format!("invalid status category: {}", s)),
        }
    }
}

impl fmt::Display for StatusCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StatusCategory::Todo => "todo",
            StatusCategory::InProgress => "in_progress",
            StatusCategory::Done => "done",
            StatusCategory::Confirmed => "confirmed",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub created_by_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: i64,
    pub workflow_id: i64,
    pub key: String,
    pub label: String,
    pub category: StatusCategory,
    pub position: i32,
    pub is_initial: bool,
    pub created_by_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransition {
    pub id: i64,
    pub workflow_id: i64,
    pub from_status_id: i64,
    pub to_status_id: i64,
    pub created_by_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDetail {
    pub workflow: Workflow,
    pub statuses: Vec<WorkflowStatus>,
    pub transitions: Vec<WorkflowTransition>,
}
