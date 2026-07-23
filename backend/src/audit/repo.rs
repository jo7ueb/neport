use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::error::{AppError, Result};
use super::models::AuditLogEntry;

type AuditLogRow = (
    String,
    i64,
    String,
    String,
    String,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

fn row_to_entry(row: AuditLogRow) -> AuditLogEntry {
    let (id, actor_id, actor_name, action, target_type, target_id, target_name, from_text, to_text, ip_address, occurred_at) = row;
    AuditLogEntry { id, actor_id, actor_name, action, target_type, target_id, target_name, from_text, to_text, ip_address, occurred_at }
}

// A system-wide activity feed merged from every table that records "who did
// what, when" - not just the `audit_log` table itself. Several kinds of user
// action (ticket field/status/assignee changes, comments, dependency
// add/remove, daily report approvals, monthly closures, work session
// start/end, attachment uploads, approver assignment) already have an actor
// and a timestamp column on their own domain table; rather than duplicating
// that into a second write to `audit_log`, each is normalized into the same
// shape here at read time and UNION ALL'd together. See
// backend/src/tickets/handlers.rs's record_update_activity for the seven
// field_name values field_update rows can carry (kept as an explicit CASE
// below so each gets its own translated label; an unrecognized future value
// falls back to a generic "ticket_field_changed" rather than being dropped).
const ENTRIES_SQL: &str = "(
    SELECT 'audit-' || a.id::text AS id, a.actor_id, actor.display_name AS actor_name, a.action,
        a.target_type, a.target_id,
        COALESCE(tu.display_name, tp.name, tt.title, tw.name, tm.name) AS target_name,
        a.from_text, a.to_text, a.ip_address, a.occurred_at
    FROM audit_log a
    JOIN users actor ON actor.id = a.actor_id
    LEFT JOIN users tu ON tu.id = a.target_id AND a.target_type IN ('user', 'user_approver', 'project_member')
    LEFT JOIN projects tp ON tp.id = a.target_id AND a.target_type = 'project'
    LEFT JOIN tickets tt ON tt.id = a.target_id AND a.target_type = 'ticket'
    LEFT JOIN workflows tw ON tw.id = a.target_id AND a.target_type = 'workflow'
    LEFT JOIN milestones tm ON tm.id = a.target_id AND a.target_type = 'milestone'
)
UNION ALL
(
    SELECT 'ticket_activity-' || t.id::text, t.user_id, actor.display_name,
        CASE t.activity_type
            WHEN 'created' THEN 'ticket_created'
            WHEN 'status_change' THEN 'ticket_status_change'
            WHEN 'assignee_change' THEN 'ticket_assignee_change'
            WHEN 'field_update' THEN COALESCE(
                CASE t.field_name
                    WHEN 'title' THEN 'ticket_title_changed'
                    WHEN 'description' THEN 'ticket_description_changed'
                    WHEN 'due_date' THEN 'ticket_due_date_changed'
                    WHEN 'start_date' THEN 'ticket_start_date_changed'
                    WHEN 'end_date' THEN 'ticket_end_date_changed'
                    WHEN 'milestone_id' THEN 'ticket_milestone_changed'
                    WHEN 'parent_ticket_id' THEN 'ticket_parent_changed'
                END,
                'ticket_field_changed'
            )
        END,
        'ticket', t.ticket_id, tk.title,
        CASE t.activity_type
            WHEN 'status_change' THEN from_ws.label
            WHEN 'assignee_change' THEN from_u.display_name
            WHEN 'field_update' THEN t.from_text
            ELSE NULL
        END,
        CASE t.activity_type
            WHEN 'status_change' THEN to_ws.label
            WHEN 'assignee_change' THEN to_u.display_name
            WHEN 'field_update' THEN t.to_text
            ELSE NULL
        END,
        NULL::text, t.occurred_at
    FROM ticket_activity_log t
    JOIN users actor ON actor.id = t.user_id
    JOIN tickets tk ON tk.id = t.ticket_id
    LEFT JOIN workflow_statuses from_ws ON from_ws.id = t.from_status_id
    LEFT JOIN workflow_statuses to_ws ON to_ws.id = t.to_status_id
    LEFT JOIN users from_u ON from_u.id = t.from_assignee_id
    LEFT JOIN users to_u ON to_u.id = t.to_assignee_id
    WHERE t.activity_type != 'comment'
)
UNION ALL
(
    SELECT 'ticket_comment-' || c.id::text, c.author_id, actor.display_name, 'ticket_commented',
        'ticket', c.ticket_id, tk.title, NULL::text,
        LEFT(c.body, 80) || CASE WHEN LENGTH(c.body) > 80 THEN '…' ELSE '' END,
        NULL::text, c.created_at
    FROM ticket_comments c
    JOIN users actor ON actor.id = c.author_id
    JOIN tickets tk ON tk.id = c.ticket_id
)
UNION ALL
(
    SELECT 'dependency_added-' || d.id::text, d.created_by_id, actor.display_name, 'dependency_added',
        'ticket', d.successor_ticket_id, succ.title,
        NULL::text, '#' || pred.id::text || ' ' || pred.title,
        NULL::text, d.created_at
    FROM ticket_dependencies d
    JOIN users actor ON actor.id = d.created_by_id
    JOIN tickets succ ON succ.id = d.successor_ticket_id
    JOIN tickets pred ON pred.id = d.predecessor_ticket_id
    WHERE d.created_by_id IS NOT NULL
)
UNION ALL
(
    SELECT 'dependency_removed-' || d.id::text, d.deleted_by_id, actor.display_name, 'dependency_removed',
        'ticket', d.successor_ticket_id, succ.title,
        NULL::text, '#' || pred.id::text || ' ' || pred.title,
        NULL::text, d.deleted_at
    FROM ticket_dependencies d
    JOIN users actor ON actor.id = d.deleted_by_id
    JOIN tickets succ ON succ.id = d.successor_ticket_id
    JOIN tickets pred ON pred.id = d.predecessor_ticket_id
    WHERE d.deleted_at IS NOT NULL AND d.deleted_by_id IS NOT NULL
)
UNION ALL
(
    SELECT 'daily_approval-' || a.id::text, a.approver_id, actor.display_name, 'report_' || a.decision,
        'user', a.user_id, subject.display_name, NULL::text, a.report_date::text,
        NULL::text, a.decided_at
    FROM daily_approvals a
    JOIN users actor ON actor.id = a.approver_id
    JOIN users subject ON subject.id = a.user_id
)
UNION ALL
(
    SELECT 'monthly_closure-' || m.id::text, m.actor_id, actor.display_name, 'month_' || m.action,
        'user', m.user_id, subject.display_name, NULL::text,
        m.year::text || '-' || LPAD(m.month::text, 2, '0'),
        NULL::text, m.acted_at
    FROM monthly_closures m
    JOIN users actor ON actor.id = m.actor_id
    JOIN users subject ON subject.id = m.user_id
)
UNION ALL
(
    SELECT 'work_session_start-' || w.id::text, w.user_id, actor.display_name, 'work_session_started',
        'user', w.user_id, actor.display_name, NULL::text, NULL::text,
        NULL::text, w.started_at
    FROM work_sessions w
    JOIN users actor ON actor.id = w.user_id
)
UNION ALL
(
    SELECT 'work_session_end-' || w.id::text, w.user_id, actor.display_name, 'work_session_ended',
        'user', w.user_id, actor.display_name, NULL::text, NULL::text,
        NULL::text, w.ended_at
    FROM work_sessions w
    JOIN users actor ON actor.id = w.user_id
    WHERE w.ended_at IS NOT NULL
)
UNION ALL
(
    SELECT 'attachment-' || a.id::text, a.uploaded_by_id, actor.display_name, 'attachment_uploaded',
        'attachment', a.id, NULL::text, NULL::text,
        a.content_type || ' (' || pg_size_pretty(a.size_bytes) || ')',
        NULL::text, a.created_at
    FROM attachments a
    JOIN users actor ON actor.id = a.uploaded_by_id
    WHERE a.uploaded_by_id IS NOT NULL
)
UNION ALL
(
    SELECT 'approver_assigned-' || u.id::text, u.assigned_by_id, actor.display_name, 'approver_assigned',
        'user', u.user_id, subject.display_name, NULL::text, approver.display_name,
        NULL::text, u.created_at
    FROM user_approvers u
    JOIN users actor ON actor.id = u.assigned_by_id
    JOIN users subject ON subject.id = u.user_id
    JOIN users approver ON approver.id = u.approver_id
    WHERE u.assigned_by_id IS NOT NULL
)";

// from/to are plain calendar dates (as offered by an <input type="date">
// picker), not timestamps - there's no single meaningful timezone for a
// system-wide log spanning every user, so the range is treated as whole UTC
// days: [from 00:00, to+1day 00:00).
pub async fn list(
    db: &PgPool,
    actor_id: Option<i64>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLogEntry>> {
    let sql = format!(
        "SELECT * FROM ({ENTRIES_SQL}) entries
         WHERE ($1::bigint IS NULL OR entries.actor_id = $1)
           AND ($2::date IS NULL OR entries.occurred_at >= $2::date)
           AND ($3::date IS NULL OR entries.occurred_at < $3::date + INTERVAL '1 day')
         ORDER BY entries.occurred_at DESC LIMIT $4 OFFSET $5"
    );
    sqlx::query_as::<_, AuditLogRow>(&sql)
        .bind(actor_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_entry).collect())
}

pub async fn count(db: &PgPool, actor_id: Option<i64>, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<i64> {
    let sql = format!(
        "SELECT COUNT(*) FROM ({ENTRIES_SQL}) entries
         WHERE ($1::bigint IS NULL OR entries.actor_id = $1)
           AND ($2::date IS NULL OR entries.occurred_at >= $2::date)
           AND ($3::date IS NULL OR entries.occurred_at < $3::date + INTERVAL '1 day')"
    );
    let (count,): (i64,) = sqlx::query_as(&sql)
        .bind(actor_id)
        .bind(from)
        .bind(to)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}

// System-wide audit trail for actions that mutate or remove an existing row
// (suspending/deleting a user, archiving a project, resetting a password,
// etc.) and that have no other table of their own to be read back from - see
// ENTRIES_SQL above for the many action kinds sourced elsewhere instead.
pub async fn record(
    db: &PgPool,
    actor_id: i64,
    action: &str,
    target_type: &str,
    target_id: Option<i64>,
    from_text: Option<&str>,
    to_text: Option<&str>,
    ip_address: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO audit_log (actor_id, action, target_type, target_id, from_text, to_text, ip_address, occurred_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(from_text)
    .bind(to_text)
    .bind(ip_address)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}
