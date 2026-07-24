use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

use crate::{auth::extractors::AuthUser, error::{AppError, Result}, pagination::{PageQuery, Paginated}, state::AppState};
use super::models::TicketComment;

pub mod handlers {
    use super::*;
    use crate::access::{self, Access};
    use crate::activity_log::{self, ActivityType};

    pub async fn list_comments(
        State(state): State<Arc<AppState>>,
        AuthUser(user): AuthUser,
        Path(ticket_id): Path<i64>,
        Query(page): Query<PageQuery>,
    ) -> Result<Json<Paginated<TicketComment>>> {
        let project_id = access::project_id_for_ticket(&state.db, ticket_id).await?;
        access::check_project_access(&state.db, &user, project_id, Access::Read).await?;
        let comments = super::list_comments(&state.db, ticket_id, page.limit(), page.offset()).await?;
        let total = super::count_comments(&state.db, ticket_id).await?;
        Ok(Json(Paginated::new(comments, total, &page)))
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateCommentRequest {
        pub body: String,
    }

    pub async fn create_comment(
        State(state): State<Arc<AppState>>,
        AuthUser(user): AuthUser,
        crate::audit::client_ip::ClientIp(ip): crate::audit::client_ip::ClientIp,
        Path(ticket_id): Path<i64>,
        Json(payload): Json<CreateCommentRequest>,
    ) -> Result<(StatusCode, Json<TicketComment>)> {
        let project_id = access::project_id_for_ticket(&state.db, ticket_id).await?;
        access::check_project_access(&state.db, &user, project_id, Access::Write).await?;
        let access_log_id = crate::audit::access_log::record(&state.db, user.id, ip.as_deref()).await?;
        let comment = super::create_comment(&state.db, ticket_id, user.id, &payload.body, access_log_id).await?;
        activity_log::repo::record(&state.db, ticket_id, user.id, ActivityType::Comment, access_log_id).await?;

        // Notify whoever has a stake in this ticket (assignee + reporter),
        // excluding the commenter themselves and de-duplicated in case the
        // reporter is also the assignee.
        if let Some(ticket) = crate::tickets::repo::get_ticket(&state.db, ticket_id).await? {
            let mut recipients = vec![ticket.reporter_id];
            if let Some(assignee_id) = ticket.assignee_id {
                recipients.push(assignee_id);
            }
            recipients.sort_unstable();
            recipients.dedup();
            for recipient_id in &recipients {
                crate::notifications::repo::notify_ticket_commented(&state.db, *recipient_id, user.id, ticket_id).await?;
            }

            // Anyone @mentioned in the comment who isn't already a stakeholder
            // gets a separate "mentioned you" notification instead of a
            // redundant second "commented" one.
            for mentioned_id in crate::mentions::extract_mentioned_user_ids(&payload.body) {
                if recipients.contains(&mentioned_id) {
                    continue;
                }
                crate::notifications::repo::notify_mentioned_in_ticket(&state.db, mentioned_id, user.id, ticket_id).await?;
            }
        }

        Ok((StatusCode::CREATED, Json(comment)))
    }
}

pub async fn create_comment(
    db: &PgPool,
    ticket_id: i64,
    author_id: i64,
    body: &str,
    access_log_id: i64,
) -> Result<TicketComment> {
    sqlx::query_as::<_, (i64, i64, i64, String, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "INSERT INTO ticket_comments (ticket_id, author_id, body, created_at, updated_at, access_log_id) VALUES ($1, $2, $3, NOW(), NULL, $4) RETURNING id, ticket_id, author_id, body, created_at, updated_at"
    )
    .bind(ticket_id)
    .bind(author_id)
    .bind(body)
    .bind(access_log_id)
    .fetch_one(db)
    .await
    .map_err(AppError::Database)
    .map(|(id, ticket_id, author_id, body, created_at, updated_at)| {
        TicketComment {
            id,
            ticket_id,
            author_id,
            body,
            created_at,
            updated_at,
        }
    })
}

pub async fn list_comments(db: &PgPool, ticket_id: i64, limit: i64, offset: i64) -> Result<Vec<TicketComment>> {
    sqlx::query_as::<_, (i64, i64, i64, String, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, ticket_id, author_id, body, created_at, updated_at FROM ticket_comments WHERE ticket_id = $1 ORDER BY created_at ASC LIMIT $2 OFFSET $3"
    )
    .bind(ticket_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(AppError::Database)
    .map(|rows| {
        rows.into_iter()
            .map(|(id, ticket_id, author_id, body, created_at, updated_at)| {
                TicketComment {
                    id,
                    ticket_id,
                    author_id,
                    body,
                    created_at,
                    updated_at,
                }
            })
            .collect()
    })
}

pub async fn count_comments(db: &PgPool, ticket_id: i64) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ticket_comments WHERE ticket_id = $1")
        .bind(ticket_id)
        .fetch_one(db)
        .await
        .map_err(AppError::Database)?;
    Ok(count)
}
