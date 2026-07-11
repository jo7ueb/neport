use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub id: i64,
    pub user_id: i64,
    pub report_date: NaiveDate,
    pub achievement: Option<String>,
    pub leftover: Option<String>,
    pub keep: Option<String>,
    pub problem: Option<String>,
    #[serde(rename = "try")]
    pub try_: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Response shape for GET /daily-reports/:date - the report plus whether it
// can currently be edited, so the frontend can show a lock notice instead of
// the edit form without needing a second request.
#[derive(Debug, Clone, Serialize)]
pub struct DailyReportView {
    #[serde(flatten)]
    pub report: DailyReport,
    pub locked: bool,
    pub lock_reason: Option<crate::approvals::models::LockReason>,
    // Tickets touched (via ticket_activity_log) on report_date, so the view
    // screen can show what the report is actually about with links back.
    pub touched_tickets: Vec<crate::tickets::models::Ticket>,
}
