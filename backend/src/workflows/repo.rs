use sqlx::PgPool;

use crate::error::{AppError, Result};
use super::models::{StatusCategory, Workflow, WorkflowDetail, WorkflowStatus, WorkflowTransition};

type WorkflowRow = (i64, String, bool, chrono::DateTime<chrono::Utc>, Option<i64>);
type StatusRow = (i64, i64, String, String, String, i32, bool, Option<i64>);
type TransitionRow = (i64, i64, i64, i64, Option<i64>);

fn row_to_workflow(row: WorkflowRow) -> Workflow {
    let (id, name, is_default, created_at, created_by_id) = row;
    Workflow { id, name, is_default, created_at, created_by_id }
}

fn row_to_status(row: StatusRow) -> WorkflowStatus {
    let (id, workflow_id, key, label, category_str, position, is_initial, created_by_id) = row;
    WorkflowStatus {
        id,
        workflow_id,
        key,
        label,
        category: category_str.parse().unwrap_or(StatusCategory::Todo),
        position,
        is_initial,
        created_by_id,
    }
}

fn row_to_transition(row: TransitionRow) -> WorkflowTransition {
    let (id, workflow_id, from_status_id, to_status_id, created_by_id) = row;
    WorkflowTransition { id, workflow_id, from_status_id, to_status_id, created_by_id }
}

pub async fn list_workflows(db: &PgPool) -> Result<Vec<Workflow>> {
    sqlx::query_as::<_, WorkflowRow>(
        "SELECT id, name, is_default, created_at, created_by_id FROM workflows ORDER BY is_default DESC, name ASC"
    )
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_workflow).collect())
}

pub async fn get_workflow(db: &PgPool, id: i64) -> Result<Option<Workflow>> {
    sqlx::query_as::<_, WorkflowRow>("SELECT id, name, is_default, created_at, created_by_id FROM workflows WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)
        .map(|row| row.map(row_to_workflow))
}

pub async fn create_workflow(db: &PgPool, name: &str, created_by_id: Option<i64>) -> Result<Workflow> {
    let result = sqlx::query_as::<_, WorkflowRow>(
        "INSERT INTO workflows (name, created_by_id) VALUES ($1, $2) RETURNING id, name, is_default, created_at, created_by_id"
    )
    .bind(name)
    .bind(created_by_id)
    .fetch_one(db)
    .await;

    match result {
        Ok(row) => Ok(row_to_workflow(row)),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint() == Some("workflows_name_key") {
                Err(AppError::BadRequest("a workflow with this name already exists".to_string()))
            } else {
                Err(AppError::Database(sqlx::Error::Database(db_err)))
            }
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn update_workflow(db: &PgPool, id: i64, name: &str) -> Result<Option<Workflow>> {
    let result = sqlx::query_as::<_, WorkflowRow>(
        "UPDATE workflows SET name = $2 WHERE id = $1 RETURNING id, name, is_default, created_at, created_by_id"
    )
    .bind(id)
    .bind(name)
    .fetch_optional(db)
    .await;

    match result {
        Ok(row) => Ok(row.map(row_to_workflow)),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint() == Some("workflows_name_key") {
                Err(AppError::BadRequest("a workflow with this name already exists".to_string()))
            } else {
                Err(AppError::Database(sqlx::Error::Database(db_err)))
            }
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn delete_workflow(db: &PgPool, id: i64) -> Result<()> {
    let result = sqlx::query("DELETE FROM workflows WHERE id = $1")
        .bind(id)
        .execute(db)
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("projects_workflow_id_fkey") => {
            Err(AppError::BadRequest(
                "this workflow is assigned to one or more projects".to_string(),
            ))
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn list_statuses(db: &PgPool, workflow_id: i64) -> Result<Vec<WorkflowStatus>> {
    sqlx::query_as::<_, StatusRow>(
        "SELECT id, workflow_id, key, label, category, position, is_initial, created_by_id FROM workflow_statuses WHERE workflow_id = $1 ORDER BY position ASC"
    )
    .bind(workflow_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_status).collect())
}

pub async fn list_transitions(db: &PgPool, workflow_id: i64) -> Result<Vec<WorkflowTransition>> {
    sqlx::query_as::<_, TransitionRow>(
        "SELECT id, workflow_id, from_status_id, to_status_id, created_by_id FROM workflow_transitions WHERE workflow_id = $1"
    )
    .bind(workflow_id)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| rows.into_iter().map(row_to_transition).collect())
}

pub async fn get_workflow_detail(db: &PgPool, workflow_id: i64) -> Result<Option<WorkflowDetail>> {
    let Some(workflow) = get_workflow(db, workflow_id).await? else {
        return Ok(None);
    };
    let statuses = list_statuses(db, workflow_id).await?;
    let transitions = list_transitions(db, workflow_id).await?;
    Ok(Some(WorkflowDetail { workflow, statuses, transitions }))
}

#[allow(clippy::too_many_arguments)]
pub async fn create_status(
    db: &PgPool,
    workflow_id: i64,
    key: &str,
    label: &str,
    category: StatusCategory,
    position: i32,
    is_initial: bool,
    created_by_id: Option<i64>,
) -> Result<WorkflowStatus> {
    let mut tx = db.begin().await.map_err(AppError::Database)?;

    if is_initial {
        sqlx::query("UPDATE workflow_statuses SET is_initial = false WHERE workflow_id = $1")
            .bind(workflow_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }

    let result = sqlx::query_as::<_, StatusRow>(
        "INSERT INTO workflow_statuses (workflow_id, key, label, category, position, is_initial, created_by_id) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, workflow_id, key, label, category, position, is_initial, created_by_id"
    )
    .bind(workflow_id)
    .bind(key)
    .bind(label)
    .bind(category.to_string())
    .bind(position)
    .bind(is_initial)
    .bind(created_by_id)
    .fetch_one(&mut *tx)
    .await;

    let row = match result {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) => {
            return if db_err.constraint() == Some("workflow_statuses_workflow_id_key_key") {
                Err(AppError::BadRequest("this status key is already used in this workflow".to_string()))
            } else {
                Err(AppError::Database(sqlx::Error::Database(db_err)))
            };
        }
        Err(e) => return Err(AppError::Database(e)),
    };

    tx.commit().await.map_err(AppError::Database)?;
    Ok(row_to_status(row))
}

pub async fn update_status(
    db: &PgPool,
    id: i64,
    key: Option<&str>,
    label: Option<&str>,
    category: Option<StatusCategory>,
    position: Option<i32>,
    is_initial: Option<bool>,
) -> Result<Option<WorkflowStatus>> {
    let mut tx = db.begin().await.map_err(AppError::Database)?;

    if is_initial == Some(true) {
        sqlx::query(
            "UPDATE workflow_statuses SET is_initial = false WHERE workflow_id = (SELECT workflow_id FROM workflow_statuses WHERE id = $1) AND id <> $1"
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    }

    let category_str = category.map(|c| c.to_string());
    let result = sqlx::query_as::<_, StatusRow>(
        "UPDATE workflow_statuses SET key = COALESCE($2, key), label = COALESCE($3, label), category = COALESCE($4, category), position = COALESCE($5, position), is_initial = COALESCE($6, is_initial) WHERE id = $1 RETURNING id, workflow_id, key, label, category, position, is_initial, created_by_id"
    )
    .bind(id)
    .bind(key)
    .bind(label)
    .bind(category_str)
    .bind(position)
    .bind(is_initial)
    .fetch_optional(&mut *tx)
    .await;

    let row = match result {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) => {
            return if db_err.constraint() == Some("workflow_statuses_workflow_id_key_key") {
                Err(AppError::BadRequest("this status key is already used in this workflow".to_string()))
            } else {
                Err(AppError::Database(sqlx::Error::Database(db_err)))
            };
        }
        Err(e) => return Err(AppError::Database(e)),
    };

    tx.commit().await.map_err(AppError::Database)?;
    Ok(row.map(row_to_status))
}

pub async fn delete_status(db: &PgPool, id: i64) -> Result<()> {
    let result = sqlx::query("DELETE FROM workflow_statuses WHERE id = $1")
        .bind(id)
        .execute(db)
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("tickets_status_id_fkey") => {
            Err(AppError::BadRequest(
                "this status is used by one or more tickets".to_string(),
            ))
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn create_transition(
    db: &PgPool,
    workflow_id: i64,
    from_status_id: i64,
    to_status_id: i64,
    created_by_id: Option<i64>,
) -> Result<WorkflowTransition> {
    let result = sqlx::query_as::<_, TransitionRow>(
        "INSERT INTO workflow_transitions (workflow_id, from_status_id, to_status_id, created_by_id) VALUES ($1, $2, $3, $4) RETURNING id, workflow_id, from_status_id, to_status_id, created_by_id"
    )
    .bind(workflow_id)
    .bind(from_status_id)
    .bind(to_status_id)
    .bind(created_by_id)
    .fetch_one(db)
    .await;

    match result {
        Ok(row) => Ok(row_to_transition(row)),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint() == Some("workflow_transitions_from_status_id_to_status_id_key") {
                Err(AppError::BadRequest("this transition already exists".to_string()))
            } else if db_err.is_check_violation() {
                Err(AppError::BadRequest("a status cannot transition to itself".to_string()))
            } else {
                Err(AppError::Database(sqlx::Error::Database(db_err)))
            }
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn delete_transition(db: &PgPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM workflow_transitions WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_default_workflow_id(db: &PgPool) -> Result<i64> {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM workflows WHERE is_default = true LIMIT 1")
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(id)
}

pub async fn get_status_category<'e>(db: impl sqlx::PgExecutor<'e>, status_id: i64) -> Result<StatusCategory> {
    let (category_str,): (String,) = sqlx::query_as("SELECT category FROM workflow_statuses WHERE id = $1")
        .bind(status_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    category_str.parse().map_err(AppError::BadRequest)
}

pub async fn is_transition_allowed<'e>(db: impl sqlx::PgExecutor<'e>, workflow_id: i64, from_status_id: i64, to_status_id: i64) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM workflow_transitions WHERE workflow_id = $1 AND from_status_id = $2 AND to_status_id = $3)"
    )
    .bind(workflow_id)
    .bind(from_status_id)
    .bind(to_status_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;
    Ok(exists)
}

// Used when creating a ticket: prefers the status explicitly marked as
// initial, falling back to whichever status sorts first by position so a
// workflow is always usable even if an admin hasn't set one yet.
pub async fn get_initial_status_id<'e>(db: impl sqlx::PgExecutor<'e>, workflow_id: i64) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM workflow_statuses WHERE workflow_id = $1 ORDER BY is_initial DESC, position ASC LIMIT 1"
    )
    .bind(workflow_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?;

    row.map(|(id,)| id)
        .ok_or_else(|| AppError::BadRequest("this workflow has no statuses defined".to_string()))
}
