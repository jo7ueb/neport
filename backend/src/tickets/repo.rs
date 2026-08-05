use chrono::NaiveDate;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::error::{AppError, Result};
use crate::workflows::models::{StatusCategory, WorkflowStatus};
use crate::workflows::repo as workflows_repo;
use super::models::{Ticket, TicketFilter, TicketNode, TicketUpdate};

// Shared by tickets::repo, tickets::dependencies and daily_reports::repo -
// every place that reads full ticket rows needs the joined workflow_statuses
// columns to build the embedded `Ticket.status`.
#[derive(sqlx::FromRow)]
pub(crate) struct TicketRow {
    pub id: i64,
    pub project_id: i64,
    pub parent_ticket_id: Option<i64>,
    pub reporter_id: i64,
    pub assignee_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub story_points: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub milestone_id: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status_id: i64,
    pub status_workflow_id: i64,
    pub status_key: String,
    pub status_label: String,
    pub status_category: String,
    pub status_position: i32,
    pub status_is_initial: bool,
    pub child_ticket_count: i64,
    pub child_done_ticket_count: i64,
}

// child_ticket_count/child_done_ticket_count count the *entire* subtree
// (children, grandchildren, ...), not just direct children - each is its own
// self-contained recursive CTE correlated to the outer `t.id`, walking
// tickets.parent_ticket_id down as far as it goes. `path` guards against a
// cycle the same way get_ticket_tree's own recursion does (parent changes
// are supposed to already reject creating one - see
// reparenting_a_ticket_into_its_own_descendant_is_rejected_as_a_cycle - but
// this is cheap insurance against a future bug or a row edited directly in
// the DB).
pub(crate) const TICKET_SELECT_COLUMNS: &str = "t.id, t.project_id, t.parent_ticket_id, t.reporter_id, t.assignee_id, t.title, t.description, t.story_points, t.start_date, t.end_date, t.due_date, t.milestone_id, t.created_at, t.updated_at, ws.id AS status_id, ws.workflow_id AS status_workflow_id, ws.key AS status_key, ws.label AS status_label, ws.category AS status_category, ws.position AS status_position, ws.is_initial AS status_is_initial, \
    (WITH RECURSIVE descendants AS ( \
        SELECT id, ARRAY[t.id, id] AS path FROM tickets WHERE parent_ticket_id = t.id AND deleted_at IS NULL \
        UNION ALL \
        SELECT c.id, d.path || c.id FROM tickets c JOIN descendants d ON c.parent_ticket_id = d.id WHERE c.deleted_at IS NULL AND NOT (c.id = ANY(d.path)) \
     ) SELECT COUNT(*) FROM descendants) AS child_ticket_count, \
    (WITH RECURSIVE descendants AS ( \
        SELECT id, ARRAY[t.id, id] AS path FROM tickets WHERE parent_ticket_id = t.id AND deleted_at IS NULL \
        UNION ALL \
        SELECT c.id, d.path || c.id FROM tickets c JOIN descendants d ON c.parent_ticket_id = d.id WHERE c.deleted_at IS NULL AND NOT (c.id = ANY(d.path)) \
     ) SELECT COUNT(*) FROM descendants dd JOIN tickets dt ON dt.id = dd.id JOIN workflow_statuses dws ON dt.status_id = dws.id WHERE dws.category IN ('done', 'confirmed')) AS child_done_ticket_count";
// The "AND t.deleted_at IS NULL" lives in the JOIN condition (not a WHERE
// clause) so this composes with every caller below, which all append their
// own "WHERE ..." after this constant.
pub(crate) const TICKET_FROM_JOIN: &str = "FROM tickets t JOIN workflow_statuses ws ON t.status_id = ws.id AND t.deleted_at IS NULL";
// Same columns as TICKET_SELECT_COLUMNS but unprefixed - for selecting from
// a CTE (e.g. the recursive ticket_tree query) whose own output columns
// already have these exact names, rather than from `tickets t`/`workflow_statuses ws` directly.
pub(crate) const TICKET_SELECT_COLUMNS_BARE: &str = "id, project_id, parent_ticket_id, reporter_id, assignee_id, title, description, story_points, start_date, end_date, due_date, milestone_id, created_at, updated_at, status_id, status_workflow_id, status_key, status_label, status_category, status_position, status_is_initial, child_ticket_count, child_done_ticket_count";

pub(crate) fn row_to_ticket(row: TicketRow) -> Ticket {
    Ticket {
        id: row.id,
        project_id: row.project_id,
        parent_ticket_id: row.parent_ticket_id,
        reporter_id: row.reporter_id,
        assignee_id: row.assignee_id,
        title: row.title,
        description: row.description,
        story_points: row.story_points,
        status: WorkflowStatus {
            id: row.status_id,
            workflow_id: row.status_workflow_id,
            key: row.status_key,
            label: row.status_label,
            category: row.status_category.parse().unwrap_or(StatusCategory::Todo),
            position: row.status_position,
            is_initial: row.status_is_initial,
            // This query doesn't select workflow_statuses.created_by_id -
            // fetch the status via workflows::repo directly if that's needed.
            created_by_id: None,
        },
        start_date: row.start_date,
        end_date: row.end_date,
        due_date: row.due_date,
        milestone_id: row.milestone_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
        child_ticket_count: row.child_ticket_count,
        child_done_ticket_count: row.child_done_ticket_count,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_ticket(
    db: &PgPool,
    project_id: i64,
    parent_ticket_id: Option<i64>,
    reporter_id: i64,
    assignee_id: Option<i64>,
    title: &str,
    description: Option<&str>,
    story_points: Option<i32>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    milestone_id: Option<i64>,
) -> Result<Ticket> {
    let (workflow_id,): (i64,) = sqlx::query_as("SELECT workflow_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;
    let status_id = workflows_repo::get_initial_status_id(db, workflow_id).await?;

    // INSERT ... RETURNING only exposes the tickets table, but row_to_ticket
    // needs the joined workflow_statuses columns too - simplest is to insert,
    // then re-fetch the row through the shared select+join below.
    let inserted_id: (i64,) = sqlx::query_as(
        "INSERT INTO tickets (project_id, parent_ticket_id, reporter_id, assignee_id, title, description, story_points, status_id, start_date, end_date, due_date, milestone_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW()) RETURNING id"
    )
    .bind(project_id)
    .bind(parent_ticket_id)
    .bind(reporter_id)
    .bind(assignee_id)
    .bind(title)
    .bind(description)
    .bind(story_points)
    .bind(status_id)
    .bind(start_date)
    .bind(end_date)
    .bind(due_date)
    .bind(milestone_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;

    get_ticket(db, inserted_id.0).await?.ok_or(AppError::InternalServerError)
}

pub async fn get_ticket(db: &PgPool, id: i64) -> Result<Option<Ticket>> {
    let sql = format!("SELECT {TICKET_SELECT_COLUMNS} {TICKET_FROM_JOIN} WHERE t.id = $1");
    sqlx::query_as::<_, TicketRow>(&sql)
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)
        .map(|row| row.map(row_to_ticket))
}

pub async fn list_tickets_for_project(
    db: &PgPool,
    project_id: i64,
    filter: &TicketFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<Ticket>> {
    let category_str = filter.category.map(|c| c.to_string());
    let order_by = filter.sort.order_by_sql();
    let sql = format!(
        "SELECT {TICKET_SELECT_COLUMNS} {TICKET_FROM_JOIN} WHERE t.project_id = $1 AND ($2::bigint IS NULL OR t.assignee_id = $2) AND ($3::bigint IS NULL OR t.status_id = $3) AND ($4::text IS NULL OR ws.category = $4) AND ($5::bool IS FALSE OR (t.due_date < CURRENT_DATE AND ws.category NOT IN ('done', 'confirmed'))) ORDER BY {order_by} LIMIT $6 OFFSET $7"
    );
    sqlx::query_as::<_, TicketRow>(&sql)
        .bind(project_id)
        .bind(filter.assignee_id)
        .bind(filter.status_id)
        .bind(category_str)
        .bind(filter.overdue)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_ticket).collect())
}

pub async fn count_tickets_for_project(db: &PgPool, project_id: i64, filter: &TicketFilter) -> Result<i64> {
    let category_str = filter.category.map(|c| c.to_string());
    let sql = format!(
        "SELECT COUNT(*) {TICKET_FROM_JOIN} WHERE t.project_id = $1 AND ($2::bigint IS NULL OR t.assignee_id = $2) AND ($3::bigint IS NULL OR t.status_id = $3) AND ($4::text IS NULL OR ws.category = $4) AND ($5::bool IS FALSE OR (t.due_date < CURRENT_DATE AND ws.category NOT IN ('done', 'confirmed')))"
    );
    let (count,): (i64,) = sqlx::query_as(&sql)
        .bind(project_id)
        .bind(filter.assignee_id)
        .bind(filter.status_id)
        .bind(category_str)
        .bind(filter.overdue)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}

pub async fn list_children(db: &PgPool, parent_id: i64) -> Result<Vec<Ticket>> {
    let sql = format!("SELECT {TICKET_SELECT_COLUMNS} {TICKET_FROM_JOIN} WHERE t.parent_ticket_id = $1 ORDER BY t.created_at ASC");
    sqlx::query_as::<_, TicketRow>(&sql)
        .bind(parent_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_ticket).collect())
}

// Backs the "#" mention picker in the frontend's Markdown editor: an empty
// query returns the most recently created tickets, otherwise it matches an
// exact id or a substring of the title. `query` is used verbatim as a plain
// value (not concatenated into SQL), so no escaping is needed beyond what
// ILIKE's own `%`/`_` wildcards already require.
pub async fn search_tickets(db: &PgPool, query: &str, limit: i64, user_id: i64, is_site_admin: bool) -> Result<Vec<Ticket>> {
    let pattern = format!("%{}%", query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    // Same project visibility rule as projects::repo::list_projects: a
    // site admin sees everything, otherwise only legacy (memberless)
    // projects or ones the caller is an explicit member of.
    let sql = format!(
        "SELECT {TICKET_SELECT_COLUMNS} {TICKET_FROM_JOIN} \
         WHERE ($1 = '' OR t.id::text = $1 OR t.title ILIKE $2) \
         AND ($4::bool OR NOT EXISTS (SELECT 1 FROM project_members pm WHERE pm.project_id = t.project_id) \
              OR EXISTS (SELECT 1 FROM project_members pm WHERE pm.project_id = t.project_id AND pm.user_id = $5)) \
         ORDER BY (t.id::text = $1) DESC, t.id DESC LIMIT $3"
    );
    sqlx::query_as::<_, TicketRow>(&sql)
        .bind(query)
        .bind(pattern)
        .bind(limit)
        .bind(is_site_admin)
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_ticket).collect())
}

pub async fn get_ticket_tree(db: &PgPool, project_id: i64) -> Result<Vec<TicketNode>> {
    let sql = format!(
        "WITH RECURSIVE ticket_tree AS (
            SELECT {TICKET_SELECT_COLUMNS}, 0 AS depth, ARRAY[t.id]::bigint[] AS path
            {TICKET_FROM_JOIN}
            WHERE t.project_id = $1 AND t.parent_ticket_id IS NULL
            UNION ALL
            SELECT {TICKET_SELECT_COLUMNS}, tt.depth + 1, tt.path || t.id
            FROM tickets t
            JOIN workflow_statuses ws ON t.status_id = ws.id AND t.deleted_at IS NULL
            JOIN ticket_tree tt ON t.parent_ticket_id = tt.id
            WHERE NOT (t.id = ANY(tt.path))
        )
        SELECT {TICKET_SELECT_COLUMNS_BARE} FROM ticket_tree ORDER BY path"
    );
    let flat_rows = sqlx::query_as::<_, TicketRow>(&sql)
        .bind(project_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)?;

    let mut ticket_map: HashMap<i64, TicketNode> = HashMap::new();
    let mut ordered_ids: Vec<i64> = Vec::new();
    let mut root_nodes = Vec::new();

    for row in flat_rows {
        let ticket = row_to_ticket(row);
        let id = ticket.id;
        let parent_ticket_id = ticket.parent_ticket_id;

        let node = TicketNode {
            ticket,
            children: Vec::new(),
        };

        if parent_ticket_id.is_none() {
            root_nodes.push(id);
        }

        ordered_ids.push(id);
        ticket_map.insert(id, node);
    }

    // `ordered_ids` is in `path` order, so a node always appears before any
    // of its descendants. Walking it in reverse guarantees a node's children
    // are always attached to it *before* the node itself is ever removed
    // from the map to be attached to its own parent — otherwise, whichever
    // of a parent/child pair a HashMap-order walk happened to process first
    // could silently drop the other (and everything under it) from the tree.
    for id in ordered_ids.into_iter().rev() {
        let parent_id = ticket_map.get(&id).and_then(|n| n.ticket.parent_ticket_id);
        if let Some(parent_id) = parent_id {
            if let Some(child_node) = ticket_map.remove(&id) {
                if let Some(parent_node) = ticket_map.get_mut(&parent_id) {
                    parent_node.children.insert(0, child_node);
                }
            }
        }
    }

    let result = root_nodes
        .into_iter()
        .filter_map(|id| ticket_map.remove(&id))
        .collect();

    Ok(result)
}

pub async fn update_ticket(db: &PgPool, id: i64, update: TicketUpdate) -> Result<Ticket> {
    let current = get_ticket(db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let title = update.title.unwrap_or(current.title);
    let description = update.description.or(current.description);
    let story_points = update.story_points.unwrap_or(current.story_points);
    let assignee_id = update.assignee_id.unwrap_or(current.assignee_id);
    let start_date = update.start_date.unwrap_or(current.start_date);
    let end_date = update.end_date.unwrap_or(current.end_date);
    let due_date = update.due_date.unwrap_or(current.due_date);
    let parent_ticket_id = update.parent_ticket_id.unwrap_or(current.parent_ticket_id);
    let milestone_id = update.milestone_id.unwrap_or(current.milestone_id);

    let status_id = match update.status_id {
        Some(new_status_id) if new_status_id != current.status.id => {
            let (workflow_id,): (i64,) = sqlx::query_as("SELECT workflow_id FROM projects WHERE id = $1")
                .bind(current.project_id)
                .fetch_one(db)
                .await
                .map_err(AppError::Database)?;
            if !workflows_repo::is_transition_allowed(db, workflow_id, current.status.id, new_status_id).await? {
                return Err(AppError::BadRequest(
                    "this status transition is not allowed by the project's workflow".to_string(),
                ));
            }

            // A ticket can't move into a "done"- or "confirmed"-category
            // status while any ticket in its subtree - not just direct
            // children - isn't at least done itself. current.child_*_count
            // already counts the whole subtree recursively (see
            // TICKET_SELECT_COLUMNS: child_done_ticket_count counts both
            // done and confirmed descendants), so this is just comparing two
            // counts.
            let (new_status_category,): (String,) =
                sqlx::query_as("SELECT category FROM workflow_statuses WHERE id = $1")
                    .bind(new_status_id)
                    .fetch_one(db)
                    .await
                    .map_err(AppError::Database)?;
            if (new_status_category == "done" || new_status_category == "confirmed")
                && current.child_done_ticket_count < current.child_ticket_count
            {
                return Err(AppError::BadRequest(
                    "all child tickets must be done before this ticket can be closed".to_string(),
                ));
            }

            new_status_id
        }
        _ => current.status.id,
    };

    sqlx::query(
        "UPDATE tickets SET title = $2, description = $3, story_points = $4, status_id = $5, assignee_id = $6, start_date = $7, end_date = $8, due_date = $9, parent_ticket_id = $10, milestone_id = $11, updated_at = NOW() WHERE id = $1"
    )
        .bind(id)
        .bind(&title)
        .bind(&description)
        .bind(story_points)
        .bind(status_id)
        .bind(assignee_id)
        .bind(start_date)
        .bind(end_date)
        .bind(due_date)
        .bind(parent_ticket_id)
        .bind(milestone_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;

    get_ticket(db, id).await?.ok_or(AppError::InternalServerError)
}

// Soft delete: mark the ticket (and every descendant, matching the cascade
// behavior a real DELETE used to give us via parent_ticket_id ON DELETE
// CASCADE) as deleted, without physically removing the row. This keeps
// ticket_activity_log/ticket_comments (ON DELETE CASCADE on ticket_id)
// intact for traceability instead of losing them at the moment of deletion.
pub async fn delete_ticket(db: &PgPool, id: i64) -> Result<()> {
    sqlx::query(
        "WITH RECURSIVE descendants AS (
            SELECT id FROM tickets WHERE id = $1
            UNION ALL
            SELECT t.id FROM tickets t JOIN descendants d ON t.parent_ticket_id = d.id
        )
        UPDATE tickets SET deleted_at = NOW() WHERE id IN (SELECT id FROM descendants)",
    )
    .bind(id)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn would_create_cycle(db: &PgPool, ticket_id: i64, new_parent_id: i64) -> Result<bool> {
    let result = sqlx::query_as::<_, (bool,)>(
        "WITH RECURSIVE ancestors AS (
            SELECT id, parent_ticket_id FROM tickets WHERE id = $1
            UNION ALL
            SELECT t.id, t.parent_ticket_id FROM tickets t
            JOIN ancestors a ON t.id = a.parent_ticket_id
        )
        SELECT EXISTS (SELECT 1 FROM ancestors WHERE id = $2)"
    )
    .bind(new_parent_id)
    .bind(ticket_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;

    Ok(result.0)
}

pub fn build_ticket_update(payload: &serde_json::Value) -> Result<TicketUpdate> {
    Ok(TicketUpdate {
        title: payload.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
        description: payload.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        story_points: payload
            .get("story_points")
            .map(|v| if v.is_null() { None } else { v.as_i64().map(|n| n as i32) }),
        status_id: payload.get("status_id").and_then(|v| v.as_i64()),
        assignee_id: payload
            .get("assignee_id")
            .map(|v| if v.is_null() { None } else { v.as_i64() }),
        start_date: payload
            .get("start_date")
            .map(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                }
            }),
        end_date: payload
            .get("end_date")
            .map(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                }
            }),
        due_date: payload
            .get("due_date")
            .map(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                }
            }),
        parent_ticket_id: payload
            .get("parent_ticket_id")
            .map(|v| if v.is_null() { None } else { v.as_i64() }),
        milestone_id: payload
            .get("milestone_id")
            .map(|v| if v.is_null() { None } else { v.as_i64() }),
    })
}

pub const ALLOWED_STORY_POINTS: [i32; 7] = [1, 2, 3, 5, 8, 13, 21];

pub fn validate_story_points(story_points: Option<i32>) -> Result<()> {
    if let Some(story_points) = story_points {
        if !ALLOWED_STORY_POINTS.contains(&story_points) {
            return Err(AppError::BadRequest("story_points must be one of 1, 2, 3, 5, 8, 13, 21".to_string()));
        }
    }
    Ok(())
}
