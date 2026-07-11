use axum::{extract::{Query, State}, Json};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::extractors::AdminUser, error::Result, pagination::{PageQuery, Paginated}, state::AppState};
use super::{models::AuditLogEntry, repo};

#[derive(Debug, Deserialize)]
pub struct ListAuditLogQuery {
    pub user_id: Option<i64>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_audit_log(
    State(state): State<Arc<AppState>>,
    AdminUser(_admin): AdminUser,
    Query(query): Query<ListAuditLogQuery>,
) -> Result<Json<Paginated<AuditLogEntry>>> {
    let page = PageQuery { page: query.page, per_page: query.per_page };
    let entries = repo::list(&state.db, query.user_id, query.from, query.to, page.limit(), page.offset()).await?;
    let total = repo::count(&state.db, query.user_id, query.from, query.to).await?;
    Ok(Json(Paginated::new(entries, total, &page)))
}
