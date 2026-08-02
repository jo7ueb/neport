#![allow(dead_code)]

use std::{fs, io};

use utoipa::{OpenApi, ToSchema};

/// Code-first OpenAPI description for the public Neport REST surface.
///
/// The route list intentionally lives beside the router inventory so that the
/// generated reference remains available without connecting to PostgreSQL.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Neport API",
        version = "0.1.0",
        description = "Neport task, work-session, daily-report, and administration API. All state-changing requests require the session cookie and X-CSRF-Token header."
    ),
    components(schemas(ApiError)),
    paths(
        doc_health,
        doc_auth_login,
        doc_auth_logout,
        doc_auth_me,
        doc_auth_password,
        doc_auth_locale,
        doc_auth_timezone_get,
        doc_auth_timezone_patch,
        doc_auth_profile,
        doc_auth_avatar_post,
        doc_auth_avatar_delete,
        doc_users_list,
        doc_users_create,
        doc_users_search,
        doc_users_avatar,
        doc_users_suspend,
        doc_users_resume,
        doc_users_reset_password,
        doc_users_role,
        doc_users_activity,
        doc_users_approver,
        doc_users_approvers_list,
        doc_users_approvers_add,
        doc_users_approvers_remove,
        doc_attachments_upload,
        doc_attachments_get,
        doc_backup_create,
        doc_backup_restore,
        doc_audit_list,
        doc_projects_list,
        doc_projects_create,
        doc_projects_get,
        doc_projects_update,
        doc_projects_archive,
        doc_projects_unarchive,
        doc_projects_workflow,
        doc_projects_tickets,
        doc_projects_ticket_tree,
        doc_milestones_list,
        doc_milestones_create,
        doc_milestones_get,
        doc_milestones_update,
        doc_milestones_delete,
        doc_milestones_burndown,
        doc_gantt_get,
        doc_members_list,
        doc_members_add,
        doc_members_update,
        doc_members_delete,
        doc_tickets_create,
        doc_tickets_search,
        doc_tickets_get,
        doc_tickets_update,
        doc_tickets_delete,
        doc_tickets_children,
        doc_dependencies_list,
        doc_dependencies_create,
        doc_dependencies_delete,
        doc_comments_list,
        doc_comments_create,
        doc_tickets_activity,
        doc_tickets_duration,
        doc_sessions_start,
        doc_sessions_end,
        doc_sessions_current,
        doc_sessions_today,
        doc_sessions_day,
        doc_sessions_monthly,
        doc_sessions_update,
        doc_reports_list,
        doc_reports_submit,
        doc_reports_today_tickets,
        doc_reports_get,
        doc_notifications_list,
        doc_notifications_unread,
        doc_notifications_read_all,
        doc_notifications_read,
        doc_approvals_subordinates,
        doc_approvals_monthly,
        doc_approvals_day,
        doc_approvals_decide,
        doc_approvals_close,
        doc_approvals_reopen,
        doc_settings_get,
        doc_settings_update,
        doc_workflows_list,
        doc_workflows_create,
        doc_workflows_get,
        doc_workflows_update,
        doc_workflows_delete,
        doc_statuses_create,
        doc_statuses_update,
        doc_statuses_delete,
        doc_transitions_create,
        doc_transitions_delete
    )
)]
pub struct ApiDoc;

#[derive(ToSchema)]
#[allow(dead_code)]
pub struct ApiError {
    /// Human-readable error message.
    pub error: String,
    /// Present for unexpected server-side failures and useful when reporting
    /// an incident to the operator.
    pub error_id: Option<String>,
}

pub fn export(path: &str) -> io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = ApiDoc::openapi()
        .to_pretty_json()
        .map_err(io::Error::other)?;
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_document_contains_core_routes() {
        let document = ApiDoc::openapi().to_pretty_json().unwrap();
        for route in [
            "\"/health\"",
            "\"/api/v1/auth/login\"",
            "\"/api/v1/projects/{project_id}/gantt\"",
            "\"/api/v1/tickets/{id}/comments\"",
            "\"/api/v1/workflows/{id}/transitions\"",
        ] {
            assert!(document.contains(route), "missing OpenAPI route: {route}");
        }
    }
}

#[utoipa::path(get, path = "/health", responses((status = 200, description = "成功")))]
fn doc_health() {}

#[utoipa::path(post, path = "/api/v1/auth/login", responses((status = 200, description = "成功")))]
fn doc_auth_login() {}

#[utoipa::path(post, path = "/api/v1/auth/logout", responses((status = 200, description = "成功")))]
fn doc_auth_logout() {}

#[utoipa::path(get, path = "/api/v1/auth/me", responses((status = 200, description = "成功")))]
fn doc_auth_me() {}

#[utoipa::path(patch, path = "/api/v1/auth/password", responses((status = 200, description = "成功")))]
fn doc_auth_password() {}

#[utoipa::path(patch, path = "/api/v1/auth/locale", responses((status = 200, description = "成功")))]
fn doc_auth_locale() {}

#[utoipa::path(get, path = "/api/v1/auth/timezone", responses((status = 200, description = "成功")))]
fn doc_auth_timezone_get() {}

#[utoipa::path(patch, path = "/api/v1/auth/timezone", responses((status = 200, description = "成功")))]
fn doc_auth_timezone_patch() {}

#[utoipa::path(patch, path = "/api/v1/auth/profile", responses((status = 200, description = "成功")))]
fn doc_auth_profile() {}

#[utoipa::path(post, path = "/api/v1/auth/avatar", responses((status = 200, description = "成功")))]
fn doc_auth_avatar_post() {}

#[utoipa::path(delete, path = "/api/v1/auth/avatar", responses((status = 200, description = "成功")))]
fn doc_auth_avatar_delete() {}

#[utoipa::path(get, path = "/api/v1/users", responses((status = 200, description = "成功")))]
fn doc_users_list() {}

#[utoipa::path(post, path = "/api/v1/users", responses((status = 200, description = "成功")))]
fn doc_users_create() {}

#[utoipa::path(get, path = "/api/v1/users/search", responses((status = 200, description = "成功")))]
fn doc_users_search() {}

#[utoipa::path(get, path = "/api/v1/users/{id}/avatar", responses((status = 200, description = "成功")))]
fn doc_users_avatar() {}

#[utoipa::path(post, path = "/api/v1/users/{id}/suspend", responses((status = 200, description = "成功")))]
fn doc_users_suspend() {}

#[utoipa::path(post, path = "/api/v1/users/{id}/resume", responses((status = 200, description = "成功")))]
fn doc_users_resume() {}

#[utoipa::path(post, path = "/api/v1/users/{id}/reset-password", responses((status = 200, description = "成功")))]
fn doc_users_reset_password() {}

#[utoipa::path(patch, path = "/api/v1/users/{id}/role", responses((status = 200, description = "成功")))]
fn doc_users_role() {}

#[utoipa::path(get, path = "/api/v1/users/{id}/activity", responses((status = 200, description = "成功")))]
fn doc_users_activity() {}

#[utoipa::path(patch, path = "/api/v1/users/{id}/is-approver", responses((status = 200, description = "成功")))]
fn doc_users_approver() {}

#[utoipa::path(get, path = "/api/v1/users/{id}/approvers", responses((status = 200, description = "成功")))]
fn doc_users_approvers_list() {}

#[utoipa::path(post, path = "/api/v1/users/{id}/approvers", responses((status = 200, description = "成功")))]
fn doc_users_approvers_add() {}

#[utoipa::path(delete, path = "/api/v1/users/{id}/approvers/{approver_id}", responses((status = 200, description = "成功")))]
fn doc_users_approvers_remove() {}

#[utoipa::path(post, path = "/api/v1/attachments", responses((status = 200, description = "成功")))]
fn doc_attachments_upload() {}

#[utoipa::path(get, path = "/api/v1/attachments/{filename}", responses((status = 200, description = "成功")))]
fn doc_attachments_get() {}

#[utoipa::path(post, path = "/api/v1/backup", responses((status = 200, description = "成功")))]
fn doc_backup_create() {}

#[utoipa::path(post, path = "/api/v1/backup/restore", responses((status = 200, description = "成功")))]
fn doc_backup_restore() {}

#[utoipa::path(get, path = "/api/v1/audit-log", responses((status = 200, description = "成功")))]
fn doc_audit_list() {}

#[utoipa::path(get, path = "/api/v1/projects", responses((status = 200, description = "成功")))]
fn doc_projects_list() {}

#[utoipa::path(post, path = "/api/v1/projects", responses((status = 200, description = "成功")))]
fn doc_projects_create() {}

#[utoipa::path(get, path = "/api/v1/projects/{id}", responses((status = 200, description = "成功")))]
fn doc_projects_get() {}

#[utoipa::path(patch, path = "/api/v1/projects/{id}", responses((status = 200, description = "成功")))]
fn doc_projects_update() {}

#[utoipa::path(post, path = "/api/v1/projects/{id}/archive", responses((status = 200, description = "成功")))]
fn doc_projects_archive() {}

#[utoipa::path(post, path = "/api/v1/projects/{id}/unarchive", responses((status = 200, description = "成功")))]
fn doc_projects_unarchive() {}

#[utoipa::path(get, path = "/api/v1/projects/{id}/workflow", responses((status = 200, description = "成功")))]
fn doc_projects_workflow() {}

#[utoipa::path(get, path = "/api/v1/projects/{id}/tickets", responses((status = 200, description = "成功")))]
fn doc_projects_tickets() {}

#[utoipa::path(get, path = "/api/v1/projects/{id}/tickets/tree", responses((status = 200, description = "成功")))]
fn doc_projects_ticket_tree() {}

#[utoipa::path(get, path = "/api/v1/projects/{project_id}/milestones", responses((status = 200, description = "成功")))]
fn doc_milestones_list() {}

#[utoipa::path(post, path = "/api/v1/projects/{project_id}/milestones", responses((status = 200, description = "成功")))]
fn doc_milestones_create() {}

#[utoipa::path(get, path = "/api/v1/projects/{project_id}/milestones/{id}", responses((status = 200, description = "成功")))]
fn doc_milestones_get() {}

#[utoipa::path(patch, path = "/api/v1/projects/{project_id}/milestones/{id}", responses((status = 200, description = "成功")))]
fn doc_milestones_update() {}

#[utoipa::path(delete, path = "/api/v1/projects/{project_id}/milestones/{id}", responses((status = 200, description = "成功")))]
fn doc_milestones_delete() {}

#[utoipa::path(get, path = "/api/v1/projects/{project_id}/milestones/{id}/burndown", responses((status = 200, description = "成功")))]
fn doc_milestones_burndown() {}

#[utoipa::path(get, path = "/api/v1/projects/{project_id}/gantt", responses((status = 200, description = "成功")))]
fn doc_gantt_get() {}

#[utoipa::path(get, path = "/api/v1/projects/{project_id}/members", responses((status = 200, description = "成功")))]
fn doc_members_list() {}

#[utoipa::path(post, path = "/api/v1/projects/{project_id}/members", responses((status = 200, description = "成功")))]
fn doc_members_add() {}

#[utoipa::path(patch, path = "/api/v1/projects/{project_id}/members/{user_id}", responses((status = 200, description = "成功")))]
fn doc_members_update() {}

#[utoipa::path(delete, path = "/api/v1/projects/{project_id}/members/{user_id}", responses((status = 200, description = "成功")))]
fn doc_members_delete() {}

#[utoipa::path(post, path = "/api/v1/tickets", responses((status = 200, description = "成功")))]
fn doc_tickets_create() {}

#[utoipa::path(get, path = "/api/v1/tickets/search", responses((status = 200, description = "成功")))]
fn doc_tickets_search() {}

#[utoipa::path(get, path = "/api/v1/tickets/{id}", responses((status = 200, description = "成功")))]
fn doc_tickets_get() {}

#[utoipa::path(patch, path = "/api/v1/tickets/{id}", responses((status = 200, description = "成功")))]
fn doc_tickets_update() {}

#[utoipa::path(delete, path = "/api/v1/tickets/{id}", responses((status = 200, description = "成功")))]
fn doc_tickets_delete() {}

#[utoipa::path(get, path = "/api/v1/tickets/{id}/children", responses((status = 200, description = "成功")))]
fn doc_tickets_children() {}

#[utoipa::path(get, path = "/api/v1/tickets/{id}/dependencies", responses((status = 200, description = "成功")))]
fn doc_dependencies_list() {}

#[utoipa::path(post, path = "/api/v1/tickets/{id}/dependencies", responses((status = 200, description = "成功")))]
fn doc_dependencies_create() {}

#[utoipa::path(delete, path = "/api/v1/tickets/{id}/dependencies/{dependency_id}", responses((status = 200, description = "成功")))]
fn doc_dependencies_delete() {}

#[utoipa::path(get, path = "/api/v1/tickets/{id}/comments", responses((status = 200, description = "成功")))]
fn doc_comments_list() {}

#[utoipa::path(post, path = "/api/v1/tickets/{id}/comments", responses((status = 200, description = "成功")))]
fn doc_comments_create() {}

#[utoipa::path(get, path = "/api/v1/tickets/{id}/activity", responses((status = 200, description = "成功")))]
fn doc_tickets_activity() {}

#[utoipa::path(get, path = "/api/v1/tickets/{id}/actual-duration", responses((status = 200, description = "成功")))]
fn doc_tickets_duration() {}

#[utoipa::path(post, path = "/api/v1/work-sessions/start", responses((status = 200, description = "成功")))]
fn doc_sessions_start() {}

#[utoipa::path(post, path = "/api/v1/work-sessions/end", responses((status = 200, description = "成功")))]
fn doc_sessions_end() {}

#[utoipa::path(get, path = "/api/v1/work-sessions/current", responses((status = 200, description = "成功")))]
fn doc_sessions_current() {}

#[utoipa::path(get, path = "/api/v1/work-sessions/today", responses((status = 200, description = "成功")))]
fn doc_sessions_today() {}

#[utoipa::path(get, path = "/api/v1/work-sessions/day", responses((status = 200, description = "成功")))]
fn doc_sessions_day() {}

#[utoipa::path(get, path = "/api/v1/work-sessions/monthly", responses((status = 200, description = "成功")))]
fn doc_sessions_monthly() {}

#[utoipa::path(patch, path = "/api/v1/work-sessions/{id}", responses((status = 200, description = "成功")))]
fn doc_sessions_update() {}

#[utoipa::path(get, path = "/api/v1/daily-reports", responses((status = 200, description = "成功")))]
fn doc_reports_list() {}

#[utoipa::path(post, path = "/api/v1/daily-reports", responses((status = 200, description = "成功")))]
fn doc_reports_submit() {}

#[utoipa::path(get, path = "/api/v1/daily-reports/today/tickets", responses((status = 200, description = "成功")))]
fn doc_reports_today_tickets() {}

#[utoipa::path(get, path = "/api/v1/daily-reports/{date}", responses((status = 200, description = "成功")))]
fn doc_reports_get() {}

#[utoipa::path(get, path = "/api/v1/notifications", responses((status = 200, description = "成功")))]
fn doc_notifications_list() {}

#[utoipa::path(get, path = "/api/v1/notifications/unread-count", responses((status = 200, description = "成功")))]
fn doc_notifications_unread() {}

#[utoipa::path(post, path = "/api/v1/notifications/read-all", responses((status = 200, description = "成功")))]
fn doc_notifications_read_all() {}

#[utoipa::path(post, path = "/api/v1/notifications/{id}/read", responses((status = 200, description = "成功")))]
fn doc_notifications_read() {}

#[utoipa::path(get, path = "/api/v1/approvals/subordinates", responses((status = 200, description = "成功")))]
fn doc_approvals_subordinates() {}

#[utoipa::path(get, path = "/api/v1/approvals/subordinates/{user_id}/monthly", responses((status = 200, description = "成功")))]
fn doc_approvals_monthly() {}

#[utoipa::path(get, path = "/api/v1/approvals/subordinates/{user_id}/day/{date}", responses((status = 200, description = "成功")))]
fn doc_approvals_day() {}

#[utoipa::path(post, path = "/api/v1/approvals/subordinates/{user_id}/day/{date}/decide", responses((status = 200, description = "成功")))]
fn doc_approvals_decide() {}

#[utoipa::path(post, path = "/api/v1/approvals/subordinates/{user_id}/month/{year}/{month}/close", responses((status = 200, description = "成功")))]
fn doc_approvals_close() {}

#[utoipa::path(post, path = "/api/v1/approvals/subordinates/{user_id}/month/{year}/{month}/reopen", responses((status = 200, description = "成功")))]
fn doc_approvals_reopen() {}

#[utoipa::path(get, path = "/api/v1/settings", responses((status = 200, description = "成功")))]
fn doc_settings_get() {}

#[utoipa::path(patch, path = "/api/v1/settings", responses((status = 200, description = "成功")))]
fn doc_settings_update() {}

#[utoipa::path(get, path = "/api/v1/workflows", responses((status = 200, description = "成功")))]
fn doc_workflows_list() {}

#[utoipa::path(post, path = "/api/v1/workflows", responses((status = 200, description = "成功")))]
fn doc_workflows_create() {}

#[utoipa::path(get, path = "/api/v1/workflows/{id}", responses((status = 200, description = "成功")))]
fn doc_workflows_get() {}

#[utoipa::path(patch, path = "/api/v1/workflows/{id}", responses((status = 200, description = "成功")))]
fn doc_workflows_update() {}

#[utoipa::path(delete, path = "/api/v1/workflows/{id}", responses((status = 200, description = "成功")))]
fn doc_workflows_delete() {}

#[utoipa::path(post, path = "/api/v1/workflows/{id}/statuses", responses((status = 200, description = "成功")))]
fn doc_statuses_create() {}

#[utoipa::path(patch, path = "/api/v1/workflows/{id}/statuses/{status_id}", responses((status = 200, description = "成功")))]
fn doc_statuses_update() {}

#[utoipa::path(delete, path = "/api/v1/workflows/{id}/statuses/{status_id}", responses((status = 200, description = "成功")))]
fn doc_statuses_delete() {}

#[utoipa::path(post, path = "/api/v1/workflows/{id}/transitions", responses((status = 200, description = "成功")))]
fn doc_transitions_create() {}

#[utoipa::path(delete, path = "/api/v1/workflows/{id}/transitions/{transition_id}", responses((status = 200, description = "成功")))]
fn doc_transitions_delete() {}
