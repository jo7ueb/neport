use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub locale: String,
    // Lightweight cache-busting marker for GET /users/:id/avatar - null
    // means no avatar is set. The image bytes themselves are never embedded
    // here (fetched separately, on demand, via that endpoint).
    pub avatar_updated_at: Option<DateTime<Utc>>,
    // Whether this user is eligible to be picked as someone's approver in
    // the work-time approval workflow (see the `approvals` module) -
    // independent of role: a general user can be an approver too.
    pub is_approver: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    General,
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "admin" => Ok(UserRole::Admin),
            "general" => Ok(UserRole::General),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

impl ToString for UserRole {
    fn to_string(&self) -> String {
        match self {
            UserRole::Admin => "admin".to_string(),
            UserRole::General => "general".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl std::str::FromStr for UserStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => Ok(UserStatus::Active),
            "suspended" => Ok(UserStatus::Suspended),
            "deleted" => Ok(UserStatus::Deleted),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

impl ToString for UserStatus {
    fn to_string(&self) -> String {
        match self {
            UserStatus::Active => "active".to_string(),
            UserStatus::Suspended => "suspended".to_string(),
            UserStatus::Deleted => "deleted".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub csrf_token: String,
}
