use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::users::repo::{row_to_user, UserRow};
use super::models::{ProjectMember, ProjectRole};

const MEMBER_COLUMNS: &str =
    "pm.id, pm.project_id, pm.role, pm.created_at, pm.added_by_id, u.id, u.email, u.display_name, u.role, u.status, u.locale, u.avatar_updated_at, u.is_approver";

type MemberRow = (i64, i64, String, chrono::DateTime<chrono::Utc>, Option<i64>, i64, String, String, String, String, String, Option<chrono::DateTime<chrono::Utc>>, bool);

fn row_to_member(row: MemberRow) -> ProjectMember {
    let (id, project_id, role_str, created_at, added_by_id, u_id, u_email, u_display_name, u_role, u_status, u_locale, u_avatar_updated_at, u_is_approver) = row;
    let user_row: UserRow = (u_id, u_email, u_display_name, u_role, u_status, u_locale, u_avatar_updated_at, u_is_approver);
    ProjectMember {
        id,
        project_id,
        user: row_to_user(user_row),
        role: role_str.parse().unwrap_or(ProjectRole::Viewer),
        created_at,
        added_by_id,
    }
}

pub async fn list_members(db: &PgPool, project_id: i64) -> Result<Vec<ProjectMember>> {
    let sql = format!(
        "SELECT {MEMBER_COLUMNS} FROM project_members pm JOIN users u ON pm.user_id = u.id WHERE pm.project_id = $1 ORDER BY pm.created_at"
    );
    sqlx::query_as::<_, MemberRow>(&sql)
        .bind(project_id)
        .fetch_all(db)
        .await
        .map_err(AppError::Database)
        .map(|rows| rows.into_iter().map(row_to_member).collect())
}

pub async fn has_any_members(db: &PgPool, project_id: i64) -> Result<bool> {
    let (exists,): (bool,) =
        sqlx::query_as("SELECT EXISTS (SELECT 1 FROM project_members WHERE project_id = $1)")
            .bind(project_id)
            .fetch_one(db)
            .await
            .map_err(AppError::Database)?;
    Ok(exists)
}

pub async fn get_role(db: &PgPool, project_id: i64, user_id: i64) -> Result<Option<ProjectRole>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT role FROM project_members WHERE project_id = $1 AND user_id = $2")
            .bind(project_id)
            .bind(user_id)
            .fetch_optional(db)
            .await
            .map_err(AppError::Database)?;
    Ok(row.map(|(r,)| r.parse().unwrap_or(ProjectRole::Viewer)))
}

pub async fn list_manager_user_ids(db: &PgPool, project_id: i64) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> =
        sqlx::query_as("SELECT user_id FROM project_members WHERE project_id = $1 AND role = 'manager'")
            .bind(project_id)
            .fetch_all(db)
            .await
            .map_err(AppError::Database)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub async fn count_managers(db: &PgPool, project_id: i64) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM project_members WHERE project_id = $1 AND role = 'manager'",
    )
    .bind(project_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;
    Ok(count)
}

pub async fn add_member(db: &PgPool, project_id: i64, user_id: i64, role: ProjectRole, added_by_id: Option<i64>) -> Result<ProjectMember> {
    let result = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO project_members (project_id, user_id, role, added_by_id) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(project_id)
    .bind(user_id)
    .bind(role.to_string())
    .bind(added_by_id)
    .fetch_one(db)
    .await;

    let id = match result {
        Ok((id,)) => id,
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("project_members_project_id_user_id_key") => {
            return Err(AppError::BadRequest("this user is already a member of this project".to_string()));
        }
        Err(e) => return Err(AppError::Database(e)),
    };

    let sql = format!(
        "SELECT {MEMBER_COLUMNS} FROM project_members pm JOIN users u ON pm.user_id = u.id WHERE pm.id = $1"
    );
    let row = sqlx::query_as::<_, MemberRow>(&sql)
        .bind(id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(row_to_member(row))
}

pub async fn update_member_role(db: &PgPool, project_id: i64, user_id: i64, role: ProjectRole) -> Result<()> {
    let result = sqlx::query("UPDATE project_members SET role = $1 WHERE project_id = $2 AND user_id = $3")
        .bind(role.to_string())
        .bind(project_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn remove_member(db: &PgPool, project_id: i64, user_id: i64) -> Result<()> {
    let result = sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
        .bind(project_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
