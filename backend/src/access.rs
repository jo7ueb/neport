use sqlx::PgPool;

use crate::auth::{User, UserRole};
use crate::error::{AppError, Result};
use crate::project_members::{models::ProjectRole, repo as members_repo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// View the project and its tickets/milestones/etc.
    Read,
    /// Create/edit/delete tickets, comments, dependencies, milestones.
    Write,
    /// Edit project settings and manage membership.
    Manage,
}

// Central authorization check for everything under a project. Site admins
// always pass. A project with no project_members rows at all is "legacy" -
// this app had zero access control before this feature existed, so an
// unmanaged project stays fully open (read+write) to any authenticated
// user rather than silently locking everyone out; only "Manage" requires
// an explicit manager (or site admin) even on a legacy project, so there's
// always a way to bootstrap membership without every user being able to
// touch project settings.
pub async fn check_project_access(db: &PgPool, user: &User, project_id: i64, required: Access) -> Result<()> {
    // Archived projects are read-only for everyone, including site admins -
    // the only way to write to one again is to unarchive it first (see
    // projects::handlers::unarchive_project, which bypasses this check
    // entirely since it doesn't operate "on" the project's own content).
    if required != Access::Read && crate::projects::repo::is_archived(db, project_id).await? {
        return Err(AppError::Forbidden);
    }

    if user.role == UserRole::Admin {
        return Ok(());
    }

    let has_members = members_repo::has_any_members(db, project_id).await?;
    if !has_members {
        return match required {
            Access::Read | Access::Write => Ok(()),
            Access::Manage => Err(AppError::Forbidden),
        };
    }

    let role = members_repo::get_role(db, project_id, user.id).await?;
    let allowed = match (role, required) {
        (Some(_), Access::Read) => true,
        (Some(ProjectRole::Manager) | Some(ProjectRole::Member), Access::Write) => true,
        (Some(ProjectRole::Manager), Access::Manage) => true,
        _ => false,
    };

    if allowed {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn project_id_for_ticket(db: &PgPool, ticket_id: i64) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT project_id FROM tickets WHERE id = $1 AND deleted_at IS NULL")
        .bind(ticket_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::Database)?;
    row.map(|(id,)| id).ok_or(AppError::NotFound)
}
