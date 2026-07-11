use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::workflows::models::{StatusCategory, WorkflowStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: i64,
    pub project_id: i64,
    pub parent_ticket_id: Option<i64>,
    pub reporter_id: i64,
    pub assignee_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub status: WorkflowStatus,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub milestone_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketNode {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub children: Vec<TicketNode>,
}

#[derive(Debug, Clone)]
pub struct TicketUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status_id: Option<i64>,
    pub assignee_id: Option<Option<i64>>,
    pub start_date: Option<Option<NaiveDate>>,
    pub end_date: Option<Option<NaiveDate>>,
    pub due_date: Option<Option<NaiveDate>>,
    pub parent_ticket_id: Option<Option<i64>>,
    pub milestone_id: Option<Option<i64>>,
}

pub struct TicketFilter {
    pub assignee_id: Option<i64>,
    pub status_id: Option<i64>,
    pub category: Option<StatusCategory>,
    pub overdue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketComment {
    pub id: i64,
    pub ticket_id: i64,
    pub author_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDependency {
    pub id: i64,
    pub predecessor_ticket_id: i64,
    pub successor_ticket_id: i64,
    pub created_at: DateTime<Utc>,
    pub created_by_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependentTicket {
    pub dependency_id: i64,
    // The link's own creation time (distinct from `ticket.created_at`,
    // which is when the other ticket itself was created) - the timeline
    // uses this to place "dependency added" events chronologically.
    pub dependency_created_at: DateTime<Utc>,
    // Who created the link, for traceability. None for rows created before
    // this column existed.
    pub dependency_created_by_id: Option<i64>,
    // Set once the link is removed (soft delete - see delete_dependency).
    // The row (and these fields) stay around after removal so the ticket
    // timeline can keep showing both the "added" and "removed" events; the
    // "current dependencies" list endpoint/UI filters these out itself.
    pub dependency_deleted_at: Option<DateTime<Utc>>,
    pub dependency_deleted_by_id: Option<i64>,
    #[serde(flatten)]
    pub ticket: Ticket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDependencies {
    pub predecessors: Vec<DependentTicket>,
    pub successors: Vec<DependentTicket>,
}
