use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::project_members::{models::ProjectRole, repo as members_repo};
use crate::workflows::repo as workflows_repo;
use super::models::{Project, ProjectTicketSummary};

type ProjectRow = (
    i64,
    String,
    Option<String>,
    i64,
    f64,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<i64>,
);

fn row_to_project(row: ProjectRow) -> Project {
    let (id, name, description, workflow_id, hours_per_story_point, created_at, archived_at, created_by_id) = row;
    Project {
        id,
        name,
        description,
        workflow_id,
        hours_per_story_point,
        created_at,
        archived_at,
        created_by_id,
    }
}

pub async fn create_project(
    db: &PgPool,
    name: &str,
    description: Option<&str>,
    workflow_id: Option<i64>,
    creator_user_id: i64,
    initial_members: &[(i64, ProjectRole)],
) -> Result<Project> {
    let workflow_id = match workflow_id {
        Some(id) => id,
        None => workflows_repo::get_default_workflow_id(db).await?,
    };
    let mut tx = db.begin().await.map_err(AppError::Database)?;

    let result = sqlx::query_as::<_, ProjectRow>(
        "INSERT INTO projects (name, description, workflow_id, created_by_id) VALUES ($1, $2, $3, $4) RETURNING id, name, description, workflow_id, hours_per_story_point, created_at, archived_at, created_by_id"
    )
    .bind(name)
    .bind(description)
    .bind(workflow_id)
    .bind(creator_user_id)
    .fetch_one(&mut *tx)
    .await;

    let project = match result {
        Ok(row) => row_to_project(row),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint() == Some("projects_name_key") {
                return Err(AppError::BadRequest("project name already exists".to_string()));
            } else {
                return Err(AppError::Database(sqlx::Error::Database(db_err)));
            }
        }
        Err(e) => return Err(AppError::Database(e)),
    };

    // Explicit initial members (picked in the creation dialog) take
    // priority; otherwise fall back to making the creator the first
    // manager, immediately putting the project into "restricted" mode (see
    // access.rs) rather than leaving it open to every user like a legacy
    // pre-membership project.
    if initial_members.is_empty() {
        members_repo::insert_member(
            &mut *tx,
            project.id,
            creator_user_id,
            ProjectRole::Manager,
            Some(creator_user_id),
        )
        .await?;
    } else {
        for (user_id, role) in initial_members {
            members_repo::insert_member(&mut *tx, project.id, *user_id, *role, Some(creator_user_id)).await?;
        }
    }

    tx.commit().await.map_err(AppError::Database)?;
    Ok(project)
}

pub async fn get_project(db: &PgPool, id: i64) -> Result<Option<Project>> {
    sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, workflow_id, hours_per_story_point, created_at, archived_at, created_by_id FROM projects WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_project))
}

// Visible projects are: any if the caller is a site admin, otherwise a
// project with no members at all (legacy/unrestricted - see access.rs) or
// one the caller is an explicit member of. $1 = is_site_admin, $2 = user_id
// in both queries below, so the two callers can share this clause verbatim.
const VISIBILITY_CLAUSE: &str = "($1::bool \
    OR NOT EXISTS (SELECT 1 FROM project_members pm WHERE pm.project_id = p.id) \
    OR EXISTS (SELECT 1 FROM project_members pm WHERE pm.project_id = p.id AND pm.user_id = $2))";

pub async fn list_projects(
    db: &PgPool,
    archived: bool,
    user_id: i64,
    is_site_admin: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Project>> {
    let archived_clause = if archived { "AND p.archived_at IS NOT NULL" } else { "AND p.archived_at IS NULL" };
    let query = format!(
        "SELECT p.id, p.name, p.description, p.workflow_id, p.hours_per_story_point, p.created_at, p.archived_at, p.created_by_id FROM projects p \
         WHERE {VISIBILITY_CLAUSE} {archived_clause} ORDER BY p.created_at DESC LIMIT $3 OFFSET $4"
    );

    sqlx::query_as::<_, ProjectRow>(&query)
        .bind(is_site_admin)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_project).collect())
}

pub async fn count_projects(db: &PgPool, archived: bool, user_id: i64, is_site_admin: bool) -> Result<i64> {
    let archived_clause = if archived { "AND p.archived_at IS NOT NULL" } else { "AND p.archived_at IS NULL" };
    let query = format!(
        "SELECT COUNT(*) FROM projects p WHERE {VISIBILITY_CLAUSE} {archived_clause}"
    );

    let (count,): (i64,) = sqlx::query_as(&query)
        .bind(is_site_admin)
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}

pub async fn is_archived<'e>(db: impl sqlx::PgExecutor<'e>, id: i64) -> Result<bool> {
    let row: Option<(Option<chrono::DateTime<chrono::Utc>>,)> =
        sqlx::query_as("SELECT archived_at FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(AppError::Database)?;
    Ok(row.map(|(archived_at,)| archived_at.is_some()).unwrap_or(false))
}

pub async fn update_project(
    db: &PgPool,
    id: i64,
    name: Option<&str>,
    description: Option<&str>,
    hours_per_story_point: Option<f64>,
) -> Result<Option<Project>> {
    sqlx::query_as::<_, ProjectRow>(
        "UPDATE projects SET name = COALESCE($2, name), description = COALESCE($3, description), hours_per_story_point = COALESCE($4, hours_per_story_point) WHERE id = $1 RETURNING id, name, description, workflow_id, hours_per_story_point, created_at, archived_at, created_by_id"
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(hours_per_story_point)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)
    .map(|row| row.map(row_to_project))
}

pub async fn get_ticket_summary(db: &PgPool, project_id: i64) -> Result<ProjectTicketSummary> {
    let row: (Option<i64>, f64, i64) = sqlx::query_as(
        "SELECT
            SUM(t.story_points)::bigint,
            COALESCE(SUM(t.story_points)::double precision * p.hours_per_story_point, 0),
            COUNT(*) FILTER (WHERE t.story_points IS NULL)::bigint
         FROM projects p
         LEFT JOIN tickets t
           ON t.project_id = p.id
          AND t.deleted_at IS NULL
          AND NOT EXISTS (SELECT 1 FROM tickets child WHERE child.parent_ticket_id = t.id AND child.project_id = t.project_id AND child.deleted_at IS NULL)
          AND t.status_id IN (
              SELECT id FROM workflow_statuses WHERE category NOT IN ('done', 'confirmed')
          )
         WHERE p.id = $1
         GROUP BY p.id, p.hours_per_story_point"
    )
    .bind(project_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?
    .ok_or(AppError::NotFound)?;

    Ok(ProjectTicketSummary {
        remaining_story_points: row.0.unwrap_or(0),
        remaining_estimated_hours: row.1,
        unestimated_ticket_count: row.2,
    })
}

pub async fn archive_project(db: &PgPool, id: i64) -> Result<()> {
    sqlx::query("UPDATE projects SET archived_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn unarchive_project(db: &PgPool, id: i64) -> Result<()> {
    sqlx::query("UPDATE projects SET archived_at = NULL WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}
