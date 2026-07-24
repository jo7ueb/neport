use chrono::NaiveDate;
use sqlx::PgPool;

use crate::error::{AppError, Result};
use super::models::{Notification, NotificationType};

type NotificationRow = (
    i64,
    String,
    Option<i64>,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<NaiveDate>,
    Option<chrono::DateTime<chrono::Utc>>,
    chrono::DateTime<chrono::Utc>,
);

const SELECT_SQL: &str = "SELECT n.id, n.notif_type, n.actor_id, actor.display_name, n.ticket_id, t.title, n.report_date, n.read_at, n.created_at
     FROM notifications n
     LEFT JOIN users actor ON actor.id = n.actor_id
     LEFT JOIN tickets t ON t.id = n.ticket_id";

fn row_to_notification(row: NotificationRow) -> Notification {
    let (id, notif_type, actor_id, actor_name, ticket_id, ticket_title, report_date, read_at, created_at) = row;
    Notification { id, notif_type, actor_id, actor_name, ticket_id, ticket_title, report_date, read_at, created_at }
}

async fn create(
    db: &PgPool,
    user_id: i64,
    notif_type: NotificationType,
    actor_id: Option<i64>,
    ticket_id: Option<i64>,
    report_date: Option<NaiveDate>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO notifications (user_id, notif_type, actor_id, ticket_id, report_date) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(user_id)
    .bind(notif_type.as_str())
    .bind(actor_id)
    .bind(ticket_id)
    .bind(report_date)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn notify_ticket_assigned(db: &PgPool, assignee_id: i64, actor_id: i64, ticket_id: i64) -> Result<()> {
    if assignee_id == actor_id {
        return Ok(());
    }
    create(db, assignee_id, NotificationType::TicketAssigned, Some(actor_id), Some(ticket_id), None).await
}

pub async fn notify_ticket_commented(db: &PgPool, recipient_id: i64, actor_id: i64, ticket_id: i64) -> Result<()> {
    if recipient_id == actor_id {
        return Ok(());
    }
    create(db, recipient_id, NotificationType::TicketCommented, Some(actor_id), Some(ticket_id), None).await
}

pub async fn notify_report_decision(db: &PgPool, user_id: i64, approver_id: i64, date: NaiveDate, approved: bool) -> Result<()> {
    let notif_type = if approved { NotificationType::ReportApproved } else { NotificationType::ReportRejected };
    create(db, user_id, notif_type, Some(approver_id), None, Some(date)).await
}

pub async fn notify_mentioned_in_ticket(db: &PgPool, recipient_id: i64, actor_id: i64, ticket_id: i64) -> Result<()> {
    if recipient_id == actor_id {
        return Ok(());
    }
    create(db, recipient_id, NotificationType::TicketMentioned, Some(actor_id), Some(ticket_id), None).await
}

pub async fn notify_mentioned_in_report(db: &PgPool, recipient_id: i64, actor_id: i64, date: NaiveDate) -> Result<()> {
    if recipient_id == actor_id {
        return Ok(());
    }
    create(db, recipient_id, NotificationType::ReportMentioned, Some(actor_id), None, Some(date)).await
}

// A ticket reached a "done" (not yet confirmed) status - notify whoever can
// actually confirm/close it. Prefers the project's managers; a project with
// no explicit manager (legacy, or simply nobody assigned yet) falls back to
// site admins, mirroring access::Access::Manage's own fallback so "who gets
// notified" always matches "who is actually allowed to confirm".
pub async fn notify_confirmation_requested(db: &PgPool, project_id: i64, actor_id: i64, ticket_id: i64) -> Result<()> {
    let mut recipients = crate::project_members::repo::list_manager_user_ids(db, project_id).await?;
    if recipients.is_empty() {
        recipients = crate::users::repo::list_admin_ids(db).await?;
    }
    for recipient_id in recipients {
        if recipient_id == actor_id {
            continue;
        }
        create(db, recipient_id, NotificationType::TicketConfirmationRequested, Some(actor_id), Some(ticket_id), None).await?;
    }
    Ok(())
}

// Run from a periodic background job (see main.rs). Inserts one
// ticket_overdue notification per ticket the first time it's found overdue -
// the partial unique index notifications_ticket_overdue_once keeps this
// idempotent across ticks, so an assignee is only ever notified once per
// ticket rather than every time the job runs.
pub async fn notify_overdue_tickets(db: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        "INSERT INTO notifications (user_id, notif_type, ticket_id)
         SELECT t.assignee_id, $1, t.id
         FROM tickets t
         JOIN workflow_statuses ws ON t.status_id = ws.id
         WHERE t.deleted_at IS NULL
           AND t.assignee_id IS NOT NULL
           AND t.due_date IS NOT NULL
           AND t.due_date < CURRENT_DATE
           AND ws.category NOT IN ('done', 'confirmed')
         ON CONFLICT DO NOTHING"
    )
    .bind(NotificationType::TicketOverdue.as_str())
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn list_for_user(db: &PgPool, user_id: i64, unread_only: bool, limit: i64, offset: i64) -> Result<Vec<Notification>> {
    let sql = format!(
        "{SELECT_SQL} WHERE n.user_id = $1 {} ORDER BY n.created_at DESC LIMIT $2 OFFSET $3",
        if unread_only { "AND n.read_at IS NULL" } else { "" }
    );
    sqlx::query_as::<_, NotificationRow>(&sql)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_notification).collect())
}

pub async fn count_for_user(db: &PgPool, user_id: i64, unread_only: bool) -> Result<i64> {
    let sql = format!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 {}",
        if unread_only { "AND read_at IS NULL" } else { "" }
    );
    let (count,): (i64,) = sqlx::query_as(&sql)
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}

pub async fn mark_read(db: &PgPool, user_id: i64, id: i64) -> Result<()> {
    sqlx::query("UPDATE notifications SET read_at = NOW() WHERE id = $1 AND user_id = $2 AND read_at IS NULL")
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn mark_all_read(db: &PgPool, user_id: i64) -> Result<()> {
    sqlx::query("UPDATE notifications SET read_at = NOW() WHERE user_id = $1 AND read_at IS NULL")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}
