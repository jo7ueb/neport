use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

use crate::{auth::extractors::AuthUser, error::{AppError, Result}, state::AppState};
use super::models::{DependentTicket, TicketDependencies, TicketDependency};
use super::repo::{row_to_ticket, TicketRow, TICKET_FROM_JOIN, TICKET_SELECT_COLUMNS};

#[derive(sqlx::FromRow)]
struct DependentTicketRow {
    dependency_id: i64,
    dependency_created_at: chrono::DateTime<chrono::Utc>,
    dependency_created_by_id: Option<i64>,
    dependency_deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    dependency_deleted_by_id: Option<i64>,
    id: i64,
    project_id: i64,
    parent_ticket_id: Option<i64>,
    reporter_id: i64,
    assignee_id: Option<i64>,
    title: String,
    description: Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    due_date: Option<chrono::NaiveDate>,
    milestone_id: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    status_id: i64,
    status_workflow_id: i64,
    status_key: String,
    status_label: String,
    status_category: String,
    status_position: i32,
    status_is_initial: bool,
}

fn row_to_dependent_ticket(row: DependentTicketRow) -> DependentTicket {
    let ticket_row = TicketRow {
        id: row.id,
        project_id: row.project_id,
        parent_ticket_id: row.parent_ticket_id,
        reporter_id: row.reporter_id,
        assignee_id: row.assignee_id,
        title: row.title,
        description: row.description,
        start_date: row.start_date,
        end_date: row.end_date,
        due_date: row.due_date,
        milestone_id: row.milestone_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
        status_id: row.status_id,
        status_workflow_id: row.status_workflow_id,
        status_key: row.status_key,
        status_label: row.status_label,
        status_category: row.status_category,
        status_position: row.status_position,
        status_is_initial: row.status_is_initial,
    };
    DependentTicket {
        dependency_id: row.dependency_id,
        dependency_created_at: row.dependency_created_at,
        dependency_created_by_id: row.dependency_created_by_id,
        dependency_deleted_at: row.dependency_deleted_at,
        dependency_deleted_by_id: row.dependency_deleted_by_id,
        ticket: row_to_ticket(ticket_row),
    }
}

pub mod handlers {
    use super::*;
    use crate::access::{self, Access};

    pub async fn list_dependencies(
        State(state): State<Arc<AppState>>,
        AuthUser(user): AuthUser,
        Path(ticket_id): Path<i64>,
    ) -> Result<Json<TicketDependencies>> {
        let project_id = access::project_id_for_ticket(&state.db, ticket_id).await?;
        access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
        let deps = super::list_dependencies_for_ticket(&state.db, ticket_id).await?;
        Ok(Json(deps))
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateDependencyRequest {
        pub direction: String,
        pub other_ticket_id: i64,
    }

    pub async fn create_dependency(
        State(state): State<Arc<AppState>>,
        AuthUser(user): AuthUser,
        Path(ticket_id): Path<i64>,
        Json(payload): Json<CreateDependencyRequest>,
    ) -> Result<(StatusCode, Json<TicketDependency>)> {
        let project_id = access::project_id_for_ticket(&state.db, ticket_id).await?;
        access::check_project_access(&state.db, &user, project_id, Access::Write).await?;
        let (predecessor_id, successor_id) = match payload.direction.as_str() {
            "predecessor" => (payload.other_ticket_id, ticket_id),
            "successor" => (ticket_id, payload.other_ticket_id),
            _ => {
                return Err(AppError::BadRequest(
                    "direction must be 'predecessor' or 'successor'".into(),
                ))
            }
        };

        if predecessor_id == successor_id {
            return Err(AppError::BadRequest(
                "a ticket cannot depend on itself".into(),
            ));
        }

        if super::would_create_dependency_cycle(&state.db, predecessor_id, successor_id).await? {
            return Err(AppError::BadRequest(
                "this dependency would create a cycle".into(),
            ));
        }

        let dep = super::create_dependency(&state.db, predecessor_id, successor_id, user.id).await?;
        Ok((StatusCode::CREATED, Json(dep)))
    }

    pub async fn delete_dependency(
        State(state): State<Arc<AppState>>,
        AuthUser(user): AuthUser,
        Path((ticket_id, dependency_id)): Path<(i64, i64)>,
    ) -> Result<StatusCode> {
        let project_id = access::project_id_for_ticket(&state.db, ticket_id).await?;
        access::check_project_access(&state.db, &user, project_id, Access::Write).await?;
        super::delete_dependency(&state.db, dependency_id, Some(user.id)).await?;
        Ok(StatusCode::NO_CONTENT)
    }
}

pub async fn list_dependencies_for_ticket(
    db: &PgPool,
    ticket_id: i64,
) -> Result<TicketDependencies> {
    let predecessors_sql = format!(
        "SELECT td.id AS dependency_id, td.created_at AS dependency_created_at, td.created_by_id AS dependency_created_by_id, td.deleted_at AS dependency_deleted_at, td.deleted_by_id AS dependency_deleted_by_id, {TICKET_SELECT_COLUMNS}
         {TICKET_FROM_JOIN}
         JOIN ticket_dependencies td ON t.id = td.predecessor_ticket_id
         WHERE td.successor_ticket_id = $1"
    );
    let predecessors = sqlx::query_as::<_, DependentTicketRow>(&predecessors_sql)
        .bind(ticket_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(row_to_dependent_ticket)
        .collect();

    let successors_sql = format!(
        "SELECT td.id AS dependency_id, td.created_at AS dependency_created_at, td.created_by_id AS dependency_created_by_id, td.deleted_at AS dependency_deleted_at, td.deleted_by_id AS dependency_deleted_by_id, {TICKET_SELECT_COLUMNS}
         {TICKET_FROM_JOIN}
         JOIN ticket_dependencies td ON t.id = td.successor_ticket_id
         WHERE td.predecessor_ticket_id = $1"
    );
    let successors = sqlx::query_as::<_, DependentTicketRow>(&successors_sql)
        .bind(ticket_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(row_to_dependent_ticket)
        .collect();

    Ok(TicketDependencies {
        predecessors,
        successors,
    })
}

pub async fn create_dependency(
    db: &PgPool,
    predecessor_id: i64,
    successor_id: i64,
    created_by_id: i64,
) -> Result<TicketDependency> {
    sqlx::query_as::<_, (i64, i64, i64, chrono::DateTime<chrono::Utc>, Option<i64>)>(
        "INSERT INTO ticket_dependencies (predecessor_ticket_id, successor_ticket_id, created_at, created_by_id) VALUES ($1, $2, NOW(), $3) RETURNING id, predecessor_ticket_id, successor_ticket_id, created_at, created_by_id"
    )
    .bind(predecessor_id)
    .bind(successor_id)
    .bind(created_by_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(|(id, predecessor_ticket_id, successor_ticket_id, created_at, created_by_id)| {
        TicketDependency {
            id,
            predecessor_ticket_id,
            successor_ticket_id,
            created_at,
            created_by_id,
        }
    })
}

// Soft delete: the row (and its created_at/created_by_id) is kept around so
// the ticket timeline can still show the original "added" event even after
// removal - see the comment on DependentTicket.dependency_deleted_at.
pub async fn delete_dependency(db: &PgPool, dependency_id: i64, deleted_by_id: Option<i64>) -> Result<()> {
    sqlx::query("UPDATE ticket_dependencies SET deleted_at = NOW(), deleted_by_id = $2 WHERE id = $1")
        .bind(dependency_id)
        .bind(deleted_by_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn would_create_dependency_cycle(
    db: &PgPool,
    predecessor_id: i64,
    successor_id: i64,
) -> Result<bool> {
    let result = sqlx::query_as::<_, (bool,)>(
        "WITH RECURSIVE reachable AS (
            SELECT successor_ticket_id AS ticket_id FROM ticket_dependencies
            WHERE predecessor_ticket_id = $1 AND deleted_at IS NULL
            UNION
            SELECT d.successor_ticket_id FROM ticket_dependencies d
            JOIN reachable r ON d.predecessor_ticket_id = r.ticket_id
            WHERE d.deleted_at IS NULL
        )
        SELECT EXISTS (SELECT 1 FROM reachable WHERE ticket_id = $2)"
    )
    .bind(successor_id)
    .bind(predecessor_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;

    Ok(result.0)
}
