use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    TicketAssigned,
    TicketOverdue,
    TicketCommented,
    ReportApproved,
    ReportRejected,
    TicketMentioned,
    ReportMentioned,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::TicketAssigned => "ticket_assigned",
            NotificationType::TicketOverdue => "ticket_overdue",
            NotificationType::TicketCommented => "ticket_commented",
            NotificationType::ReportApproved => "report_approved",
            NotificationType::ReportRejected => "report_rejected",
            NotificationType::TicketMentioned => "ticket_mentioned",
            NotificationType::ReportMentioned => "report_mentioned",
        }
    }
}

// actor/ticket fields are resolved via JOIN at read time (not snapshotted),
// so a renamed ticket or user always shows its current title/name here -
// same tradeoff as ticket_activity_log's from_status_id/to_status_id, which
// the frontend also resolves against live data rather than a stored label.
#[derive(Debug, Clone, Serialize)]
pub struct Notification {
    pub id: i64,
    pub notif_type: String,
    pub actor_id: Option<i64>,
    pub actor_name: Option<String>,
    pub ticket_id: Option<i64>,
    pub ticket_title: Option<String>,
    pub report_date: Option<NaiveDate>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
