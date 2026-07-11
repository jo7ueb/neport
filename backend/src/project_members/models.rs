use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::auth::User;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    Manager,
    Member,
    Viewer,
}

impl FromStr for ProjectRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manager" => Ok(ProjectRole::Manager),
            "member" => Ok(ProjectRole::Member),
            "viewer" => Ok(ProjectRole::Viewer),
            _ => Err(format!("invalid project role: {}", s)),
        }
    }
}

impl fmt::Display for ProjectRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProjectRole::Manager => "manager",
            ProjectRole::Member => "member",
            ProjectRole::Viewer => "viewer",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMember {
    pub id: i64,
    pub project_id: i64,
    pub user: User,
    pub role: ProjectRole,
    pub created_at: DateTime<Utc>,
    pub added_by_id: Option<i64>,
}
