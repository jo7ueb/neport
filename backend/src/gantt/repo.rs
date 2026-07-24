use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::tickets::models::{TicketDependency, TicketFilter};
use crate::tickets::repo as tickets_repo;
use super::models::GanttData;

async fn list_dependencies_for_project(db: &PgPool, project_id: i64) -> Result<Vec<TicketDependency>> {
    sqlx::query_as::<_, (i64, i64, i64, chrono::DateTime<chrono::Utc>)>(
        "SELECT DISTINCT td.id, td.predecessor_ticket_id, td.successor_ticket_id, td.created_at
         FROM ticket_dependencies td
         WHERE td.deleted_at IS NULL
            AND (td.predecessor_ticket_id IN (SELECT id FROM tickets WHERE project_id = $1)
            OR td.successor_ticket_id IN (SELECT id FROM tickets WHERE project_id = $1))"
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| {
        rows.into_iter()
            .map(|(id, predecessor_ticket_id, successor_ticket_id, created_at)| TicketDependency {
                id,
                predecessor_ticket_id,
                successor_ticket_id,
                created_at,
                created_by_id: None,
            })
            .collect()
    })
}

pub async fn get_gantt_data(db: &PgPool, project_id: i64) -> Result<GanttData> {
    let filter = TicketFilter {
        assignee_id: None,
        status_id: None,
        category: None,
        overdue: false,
        sort: Default::default(),
    };
    // The Gantt chart needs every ticket in the project to render correctly,
    // not a paginated page, so request the internal "fetch everything" ceiling.
    let tickets = tickets_repo::list_tickets_for_project(db, project_id, &filter, crate::pagination::FETCH_ALL_LIMIT, 0).await?;
    let dependencies = list_dependencies_for_project(db, project_id).await?;

    Ok(GanttData { tickets, dependencies })
}
