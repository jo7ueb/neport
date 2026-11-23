#![allow(dead_code)]

use std::{fs, io};

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi, ToSchema,
};

/// English-first OpenAPI contract for the Neport REST API.
#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "Neport API",
        version = "0.1.0",
        description = "English-first API documentation for Neport. Every operation explains when to use it, what to send, what comes back, and how to recover from errors. Browser clients use the session cookie and X-CSRF-Token; integrations may use a scoped Bearer API token."
    ),
    tags(
        (name = "Authentication", description = "When to use: log in, restore a session after reload, and log out. Workflow: login -> retain the CSRF token -> call protected operations with the session cookie."),
        (name = "Users and permissions", description = "When to use: manage accounts, roles, approvers, and profile data. Workflow: find a user -> verify role and project membership -> perform the permitted change."),
        (name = "Projects and members", description = "When to use: create a project and its team. Workflow: create project -> configure workflow -> add members -> create tickets."),
        (name = "Tickets and comments", description = "When to use: track work and discussion. Workflow: create ticket -> assign/status it -> add comments -> inspect activity."),
        (name = "Dependencies and planning", description = "When to use: express ordering and forecast delivery. Workflow: add dependency edges -> inspect milestones, burndown, or Gantt -> remove obsolete edges."),
        (name = "Work sessions", description = "When to use: record actual time. Workflow: start one timer -> stop it -> review day/month totals. A user may have only one open session."),
        (name = "Daily reports", description = "When to use: submit a daily reflection. Workflow: write the report -> submit it -> wait for the approver; closed months cannot be edited."),
        (name = "Approvals and month closing", description = "When to use: review subordinate work and lock accounting periods. Workflow: approve every report -> close the month -> reopen only to correct an error."),
        (name = "Notifications", description = "When to use: present mention, assignment, and approval events. Workflow: fetch unread count -> list notifications -> mark them read after display."),
        (name = "Administration and backup", description = "When to use: configure workflows, audit access, and protect data. Workflow: export a backup before risky changes; restore only during maintenance and verify the result."),
        (name = "System", description = "When to use: monitor service availability. The health endpoint is safe to call without a user session.")
    ),
    components(schemas(ApiError, PaginationMeta, LoginRequest, LoginResponse, ApiTokenCreateRequest, ApiTokenInfo, ProjectCreateRequest, TicketCreateRequest, TicketUpdateRequest, DependencyCreateRequest, CommentCreateRequest, WorkSessionRequest, DailyReportRequest, MonthDecisionRequest)),
    paths(
        health_check,
        login,
        logout,
        restore_session,
        list_api_tokens,
        create_api_token,
        revoke_api_token,
        change_password,
        update_locale,
        get_timezone,
        update_timezone,
        update_profile,
        upload_avatar,
        delete_avatar,
        list_users,
        create_user,
        search_users,
        get_user_avatar,
        suspend_user,
        resume_user,
        reset_password,
        change_role,
        user_activity,
        set_approver,
        list_approvers,
        assign_approver,
        remove_approver,
        upload_attachment,
        download_attachment,
        create_backup,
        restore_backup,
        list_audit_log,
        list_projects,
        create_project,
        get_project,
        update_project,
        archive_project,
        unarchive_project,
        get_project_workflow,
        list_project_tickets,
        get_ticket_tree,
        list_milestones,
        create_milestone,
        get_milestone,
        update_milestone,
        delete_milestone,
        get_burndown,
        get_gantt,
        list_members,
        add_member,
        update_member,
        remove_member,
        create_ticket,
        search_tickets,
        get_ticket,
        update_ticket,
        delete_ticket,
        list_children,
        list_dependencies,
        create_dependency,
        delete_dependency,
        list_comments,
        create_comment,
        list_ticket_activity,
        get_actual_duration,
        start_work_session,
        end_work_session,
        get_current_work_session,
        get_today_work,
        get_day_work,
        get_monthly_work,
        update_work_session,
        list_daily_reports,
        submit_daily_report,
        get_today_report_tickets,
        get_daily_report,
        list_notifications,
        get_unread_count,
        mark_all_notifications_read,
        mark_notification_read,
        list_subordinates,
        get_monthly_review,
        get_day_review,
        decide_daily_report,
        close_month,
        reopen_month,
        get_settings,
        update_settings,
        list_workflows,
        create_workflow,
        get_workflow,
        update_workflow,
        delete_workflow,
        create_status,
        update_status,
        delete_status,
        create_transition,
        delete_transition
    )
)]
pub struct ApiDoc;

/// Adds the cookie and CSRF header schemes used by the running Axum service.
/// Individual operations explain whether these credentials are required.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "session_cookie",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("neport_session"))),
            );
            components.add_security_scheme(
                "csrf_token",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-CSRF-Token"))),
            );
            components.add_security_scheme(
                "bearer_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

#[derive(ToSchema)]
pub struct ApiError {
    /// Human-readable recovery guidance.
    pub error: String,
    /// Include this identifier when reporting an unexpected server failure.
    pub error_id: Option<String>,
}

#[derive(ToSchema)]
pub struct PaginationMeta {
    /// Items returned in this response.
    pub items: Vec<serde_json::Value>,
    /// Total number of matching items.
    pub total: i64,
    /// One-based page number.
    pub page: i64,
    /// Number of items requested per page.
    pub per_page: i64,
}

#[derive(ToSchema)]
pub struct LoginRequest {
    /// Account email address.
    #[schema(example = "user@example.com")]
    pub email: String,
    /// Password for the account.
    #[schema(format = Password, example = "correct-horse-battery-staple")]
    pub password: String,
}

#[derive(ToSchema)]
pub struct ApiTokenCreateRequest {
    /// Human-readable label for this token.
    #[schema(example = "Claude Desktop")]
    pub name: String,
    /// Allowed scopes. `read` is required; add `write` for mutations.
    #[schema(example = json!( ["read", "write"] ))]
    pub scopes: Vec<String>,
}

#[derive(ToSchema)]
pub struct ApiTokenInfo {
    /// Token identifier used for revocation. The secret is never returned here.
    pub id: uuid::Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(ToSchema)]
pub struct LoginResponse {
    /// Authenticated user identifier.
    #[schema(example = 42)]
    pub user_id: i64,
    /// CSRF token to send on state-changing requests.
    pub csrf_token: String,
}

#[derive(ToSchema)]
pub struct ProjectCreateRequest {
    /// Human-readable project name.
    #[schema(min_length = 1, max_length = 200, example = "Migration project")]
    pub name: String,
    /// Optional project description.
    #[schema(max_length = 5000, example = "Move the team from Redmine")]
    pub description: Option<String>,
}

#[derive(ToSchema)]
pub struct TicketCreateRequest {
    /// Project that will own the ticket.
    #[schema(example = 42)]
    pub project_id: i64,
    /// Short ticket title.
    #[schema(min_length = 1, max_length = 300, example = "Implement API docs")]
    pub title: String,
    /// Markdown/plain-text description.
    pub description: Option<String>,
    /// Optional assignee.
    pub assignee_id: Option<i64>,
    /// Optional active parent ticket in the same project.
    pub parent_ticket_id: Option<i64>,
    /// Optional active milestone in the same project.
    pub milestone_id: Option<i64>,
}

#[derive(ToSchema)]
pub struct TicketUpdateRequest {
    /// Ticket's updated_at from the previous read. Omit for an unconditional patch.
    #[schema(example = "2026-11-23T12:00:00.123456Z")]
    pub expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// New title, when changing the title.
    pub title: Option<String>,
    /// New description, when changing the description.
    pub description: Option<String>,
    /// New workflow status identifier.
    pub status_id: Option<i64>,
    /// New assignee identifier; null clears the assignee.
    pub assignee_id: Option<i64>,
    /// Active parent ticket in the same project; null clears the parent.
    pub parent_ticket_id: Option<i64>,
    /// Active milestone in the same project; null clears the milestone.
    pub milestone_id: Option<i64>,
}

#[derive(ToSchema)]
pub struct DependencyCreateRequest {
    /// Whether the other ticket is a predecessor or successor of the path ticket.
    #[schema(example = "predecessor")]
    pub direction: String,
    /// Another active ticket in the same project.
    #[schema(example = 17)]
    pub other_ticket_id: i64,
}

#[derive(ToSchema)]
pub struct CommentCreateRequest {
    /// Comment body. The wire format calls this field `content`.
    #[schema(
        min_length = 1,
        max_length = 10000,
        example = "The migration is ready for review."
    )]
    pub content: String,
}

#[derive(ToSchema)]
pub struct WorkSessionRequest {
    /// Optional note attached to the session.
    pub note: Option<String>,
}

#[derive(ToSchema)]
pub struct DailyReportRequest {
    /// Calendar date in `YYYY-MM-DD` format.
    #[schema(format = Date, example = "2026-08-02")]
    pub date: String,
    /// What went well.
    pub good: Option<String>,
    /// What should improve.
    pub better: Option<String>,
    /// What to try next.
    #[serde(rename = "try")]
    pub try_next: Option<String>,
    /// Problems or blockers.
    pub problem: Option<String>,
}

#[derive(ToSchema)]
pub struct MonthDecisionRequest {
    /// Approval decision. Allowed values are `approve` and `reject`.
    #[schema(example = "approve", pattern = "^(approve|reject)$")]
    pub decision: String,
    /// Optional explanation required for a rejection.
    pub comment: Option<String>,
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
    fn generated_document_is_english_and_categorized() {
        let document = ApiDoc::openapi().to_pretty_json().unwrap();
        assert!(document.contains("When to use"));
        assert!(document.contains("Authentication"));
        assert!(document.contains("create_ticket"));
        assert!(!document.contains("doc_"));
    }
}

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "health_check",
    tag = "System",
    summary = "Health check",
    description = "### When to use\nHealth check. Use this endpoint as part of the system workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn health_check() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    operation_id = "login",
    tag = "Authentication",
    summary = "Create a user session",
    request_body = LoginRequest,
    description = "### When to use\nCreate a database-backed session before calling any protected endpoint.\n\n### Request\n```json\n{\"email\":\"user@example.com\",\"password\":\"Password123\"}\n```\n\n### Response\nReturns the user, a CSRF token, and a Secure HttpOnly session cookie. Keep the CSRF token in memory and send it on every POST, PATCH, and DELETE request.\n\n### Errors\n**401** means the email/password pair is invalid. Do not reveal which field was wrong; ask the user to retry. **429** means the login rate limit was exceeded; wait and retry with exponential backoff.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn login() {}

#[utoipa::path(
    get,
    path = "/api/v1/auth/tokens",
    operation_id = "list_api_tokens",
    tag = "Authentication",
    summary = "List personal API tokens",
    description = "### When to use\nReview the caller's API tokens before connecting an MCP client. Secret token values are never returned.\n\n### Errors\n**401** means the session is missing or expired; log in again.",
    responses((status = 200, body = [ApiTokenInfo], description = "Token metadata without secrets."), (status = 401, description = "Log in again."))
)]
fn list_api_tokens() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/tokens",
    operation_id = "create_api_token",
    tag = "Authentication",
    summary = "Create a personal API token",
    request_body = ApiTokenCreateRequest,
    description = "### When to use\nCreate a token for neport-mcp or another trusted integration. The plaintext token is returned exactly once; store it in a secret manager.\n\n### Scopes\n`read` permits safe queries. Add `write` only when the integration must mutate Neport.\n\n### Errors\n**400** correct the name or scopes. **403** send the current CSRF token. **401** log in again.",
    responses((status = 201, description = "Created; the plaintext token is included once."), (status = 400, description = "Correct the token name or scopes."), (status = 401, description = "Log in again."), (status = 403, description = "Send the current CSRF token."))
)]
fn create_api_token() {}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/tokens/{id}",
    operation_id = "revoke_api_token",
    tag = "Authentication",
    summary = "Revoke a personal API token",
    params(("id" = uuid::Uuid, Path, description = "Token identifier")),
    description = "### When to use\nImmediately disable a lost, rotated, or no-longer-needed integration token. Revocation is permanent.\n\n### Errors\n**404** verify the token identifier belongs to the current user. **403** send the current CSRF token.",
    responses((status = 204, description = "Token revoked."), (status = 401, description = "Log in again."), (status = 403, description = "Send the current CSRF token."), (status = 404, description = "Verify the token identifier."))
)]
fn revoke_api_token() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    operation_id = "logout",
    tag = "Authentication",
    summary = "End the current session",
    description = "### When to use\nEnd the current session. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn logout() {}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    operation_id = "restore_session",
    tag = "Authentication",
    summary = "Restore the current session",
    description = "### When to use\nCall this once when the application starts or after a page reload. It restores the user and the current CSRF token from the session cookie.\n\n### Errors\n**401** means the cookie is missing or expired. Clear local session state and show the login screen.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn restore_session() {}

#[utoipa::path(
    patch,
    path = "/api/v1/auth/password",
    operation_id = "change_password",
    tag = "Authentication",
    summary = "Change your password",
    description = "### When to use\nChange your password. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn change_password() {}

#[utoipa::path(
    patch,
    path = "/api/v1/auth/locale",
    operation_id = "update_locale",
    tag = "Authentication",
    summary = "Update the display language",
    description = "### When to use\nUpdate the display language. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_locale() {}

#[utoipa::path(
    get,
    path = "/api/v1/auth/timezone",
    operation_id = "get_timezone",
    tag = "Authentication",
    summary = "Get the effective timezone",
    description = "### When to use\nGet the effective timezone. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_timezone() {}

#[utoipa::path(
    patch,
    path = "/api/v1/auth/timezone",
    operation_id = "update_timezone",
    tag = "Authentication",
    summary = "Set the personal timezone",
    description = "### When to use\nSet the personal timezone. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_timezone() {}

#[utoipa::path(
    patch,
    path = "/api/v1/auth/profile",
    operation_id = "update_profile",
    tag = "Authentication",
    summary = "Update the profile",
    description = "### When to use\nUpdate the profile. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_profile() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/avatar",
    operation_id = "upload_avatar",
    tag = "Authentication",
    summary = "Upload the current user's avatar",
    description = "### When to use\nUpload the current user's avatar. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn upload_avatar() {}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/avatar",
    operation_id = "delete_avatar",
    tag = "Authentication",
    summary = "Delete the current user's avatar",
    description = "### When to use\nDelete the current user's avatar. Use this endpoint as part of the authentication workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_avatar() {}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    operation_id = "list_users",
    tag = "Users and permissions",
    summary = "List users",
    description = "### When to use\nList users. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_users() {}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    operation_id = "create_user",
    tag = "Users and permissions",
    summary = "Create a user",
    description = "### When to use\nCreate a user. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_user() {}

#[utoipa::path(
    get,
    path = "/api/v1/users/search",
    operation_id = "search_users",
    tag = "Users and permissions",
    summary = "Search users",
    description = "### When to use\nSearch users. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn search_users() {}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/avatar",
    operation_id = "get_user_avatar",
    tag = "Users and permissions",
    summary = "Get a user's avatar",
    description = "### When to use\nGet a user's avatar. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_user_avatar() {}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/suspend",
    operation_id = "suspend_user",
    tag = "Users and permissions",
    summary = "Suspend a user",
    description = "### When to use\nSuspend a user. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn suspend_user() {}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/resume",
    operation_id = "resume_user",
    tag = "Users and permissions",
    summary = "Resume a user",
    description = "### When to use\nResume a user. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn resume_user() {}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/reset-password",
    operation_id = "reset_password",
    tag = "Users and permissions",
    summary = "Reset a user's password",
    description = "### When to use\nReset a user's password. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn reset_password() {}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/role",
    operation_id = "change_role",
    tag = "Users and permissions",
    summary = "Change a user's role",
    description = "### When to use\nChange a user's role. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn change_role() {}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/activity",
    operation_id = "user_activity",
    tag = "Users and permissions",
    summary = "List a user's activity",
    description = "### When to use\nList a user's activity. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn user_activity() {}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/is-approver",
    operation_id = "set_approver",
    tag = "Users and permissions",
    summary = "Set the approver flag",
    description = "### When to use\nSet the approver flag. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn set_approver() {}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/approvers",
    operation_id = "list_approvers",
    tag = "Users and permissions",
    summary = "List assigned approvers",
    description = "### When to use\nList assigned approvers. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_approvers() {}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/approvers",
    operation_id = "assign_approver",
    tag = "Users and permissions",
    summary = "Assign an approver",
    description = "### When to use\nAssign an approver. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn assign_approver() {}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/approvers/{approver_id}",
    operation_id = "remove_approver",
    tag = "Users and permissions",
    summary = "Remove an approver",
    description = "### When to use\nRemove an approver. Use this endpoint as part of the users and permissions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" ), ("approver_id" = i64, Path, description = "Resource identifier: approver_id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn remove_approver() {}

#[utoipa::path(
    post,
    path = "/api/v1/attachments",
    operation_id = "upload_attachment",
    tag = "Administration and backup",
    summary = "Upload an attachment",
    description = "### When to use\nUpload an attachment. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn upload_attachment() {}

#[utoipa::path(
    get,
    path = "/api/v1/attachments/{filename}",
    operation_id = "download_attachment",
    tag = "Administration and backup",
    summary = "Download an attachment",
    description = "### When to use\nDownload an attachment. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("filename" = i64, Path, description = "Resource identifier: filename" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn download_attachment() {}

#[utoipa::path(
    post,
    path = "/api/v1/backup",
    operation_id = "create_backup",
    tag = "Administration and backup",
    summary = "Create a backup archive",
    description = "### When to use\nCreate an administrator-only archive containing database data and uploaded files. Treat the returned binary as sensitive and store it securely.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_backup() {}

#[utoipa::path(
    post,
    path = "/api/v1/backup/restore",
    operation_id = "restore_backup",
    tag = "Administration and backup",
    summary = "Restore a backup archive",
    description = "### When to use\nRestore an administrator-only backup archive. Restore is deliberately available only during maintenance mode. Stop all writes and verify the archive before calling this endpoint.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn restore_backup() {}

#[utoipa::path(
    get,
    path = "/api/v1/audit-log",
    operation_id = "list_audit_log",
    tag = "Administration and backup",
    summary = "List audit log entries",
    description = "### When to use\nList audit log entries. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_audit_log() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects",
    operation_id = "list_projects",
    tag = "Projects and members",
    summary = "List projects",
    description = "### When to use\nPopulate a project picker or dashboard. The result is limited to projects visible to the current user.\n\n### Query parameters\nUse `archived`, `page`, and `per_page`. See the pagination rules in the response schema.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_projects() {}

#[utoipa::path(
    post,
    path = "/api/v1/projects",
    operation_id = "create_project",
    tag = "Projects and members",
    summary = "Create a project",
    request_body = ProjectCreateRequest,
    description = "### When to use\nCreate the project container before creating tickets, milestones, or members.\n\n### Request example\n```json\n{\"name\":\"Migration project\",\"description\":\"Move the team from Redmine\"}\n```\n\nThe creator becomes a project manager. **400** means the name is missing or invalid; correct the request and retry. **403** means the caller cannot create projects.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_project() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{id}",
    operation_id = "get_project",
    tag = "Projects and members",
    summary = "Get a project",
    description = "### When to use\nGet a project. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_project() {}

#[utoipa::path(
    patch,
    path = "/api/v1/projects/{id}",
    operation_id = "update_project",
    tag = "Projects and members",
    summary = "Update a project",
    description = "### When to use\nUpdate a project. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_project() {}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{id}/archive",
    operation_id = "archive_project",
    tag = "Projects and members",
    summary = "Archive a project",
    description = "### When to use\nArchive a project. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn archive_project() {}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{id}/unarchive",
    operation_id = "unarchive_project",
    tag = "Projects and members",
    summary = "Unarchive a project",
    description = "### When to use\nUnarchive a project. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn unarchive_project() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{id}/workflow",
    operation_id = "get_project_workflow",
    tag = "Projects and members",
    summary = "Get a project's workflow",
    description = "### When to use\nGet a project's workflow. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_project_workflow() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{id}/tickets",
    operation_id = "list_project_tickets",
    tag = "Tickets and comments",
    summary = "List project tickets",
    description = "### When to use\nList project tickets. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_project_tickets() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{id}/tickets/tree",
    operation_id = "get_ticket_tree",
    tag = "Tickets and comments",
    summary = "Get the ticket tree",
    description = "### When to use\nGet the ticket tree. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_ticket_tree() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/milestones",
    operation_id = "list_milestones",
    tag = "Dependencies and planning",
    summary = "List milestones",
    description = "### When to use\nList milestones. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_milestones() {}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{project_id}/milestones",
    operation_id = "create_milestone",
    tag = "Dependencies and planning",
    summary = "Create a milestone",
    description = "### When to use\nCreate a milestone. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_milestone() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/milestones/{id}",
    operation_id = "get_milestone",
    tag = "Dependencies and planning",
    summary = "Get a milestone",
    description = "### When to use\nGet a milestone. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" ), ("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_milestone() {}

#[utoipa::path(
    patch,
    path = "/api/v1/projects/{project_id}/milestones/{id}",
    operation_id = "update_milestone",
    tag = "Dependencies and planning",
    summary = "Update a milestone",
    description = "### When to use\nUpdate a milestone. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" ), ("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_milestone() {}

#[utoipa::path(
    delete,
    path = "/api/v1/projects/{project_id}/milestones/{id}",
    operation_id = "delete_milestone",
    tag = "Dependencies and planning",
    summary = "Delete a milestone",
    description = "### When to use\nDelete a milestone. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" ), ("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_milestone() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/milestones/{id}/burndown",
    operation_id = "get_burndown",
    tag = "Dependencies and planning",
    summary = "Get milestone burndown data",
    description = "### When to use\nGet milestone burndown data. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" ), ("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_burndown() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/gantt",
    operation_id = "get_gantt",
    tag = "Dependencies and planning",
    summary = "Get Gantt data",
    description = "### When to use\nGet Gantt data. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_gantt() {}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/members",
    operation_id = "list_members",
    tag = "Projects and members",
    summary = "List project members",
    description = "### When to use\nList project members. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_members() {}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{project_id}/members",
    operation_id = "add_member",
    tag = "Projects and members",
    summary = "Add a project member",
    description = "### When to use\nAdd a project member. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn add_member() {}

#[utoipa::path(
    patch,
    path = "/api/v1/projects/{project_id}/members/{user_id}",
    operation_id = "update_member",
    tag = "Projects and members",
    summary = "Update a member role",
    description = "### When to use\nUpdate a member role. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" ), ("user_id" = i64, Path, description = "Resource identifier: user_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_member() {}

#[utoipa::path(
    delete,
    path = "/api/v1/projects/{project_id}/members/{user_id}",
    operation_id = "remove_member",
    tag = "Projects and members",
    summary = "Remove a project member",
    description = "### When to use\nRemove a project member. Use this endpoint as part of the projects and members workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("project_id" = i64, Path, description = "Resource identifier: project_id" ), ("user_id" = i64, Path, description = "Resource identifier: user_id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn remove_member() {}

#[utoipa::path(
    post,
    path = "/api/v1/tickets",
    operation_id = "create_ticket",
    tag = "Tickets and comments",
    summary = "Create a ticket",
    request_body = TicketCreateRequest,
    description = "### When to use\nCreate a task in a project. `project_id` and `title` are required.\n\n### Request example\n```json\n{\"project_id\":42,\"title\":\"Implement API docs\",\"description\":\"Add examples and error guidance\",\"assignee_id\":12}\n```\n\nOptional fields include `parent_ticket_id`, `status_id`, `start_date`, `end_date`, `due_date`, and `milestone_id`. A new ticket starts in the workflow's initial status. **400** also covers parent cycles and invalid workflow transitions.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_ticket() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/search",
    operation_id = "search_tickets",
    tag = "Tickets and comments",
    summary = "Search tickets",
    description = "### When to use\nSearch tickets. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    params(("q" = Option<String>, Query, description = "Ticket ID or title text"),
           ("project_id" = Option<i64>, Query, description = "Limit results to this project")),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn search_tickets() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/{id}",
    operation_id = "get_ticket",
    tag = "Tickets and comments",
    summary = "Get a ticket",
    description = "### When to use\nGet a ticket. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_ticket() {}

#[utoipa::path(
    patch,
    path = "/api/v1/tickets/{id}",
    operation_id = "update_ticket",
    tag = "Tickets and comments",
    summary = "Update a ticket",
    request_body = TicketUpdateRequest,
    description = "### When to use\nPartially update a ticket. Send only fields that changed. Include the exact updated_at value from the ticket as expected_updated_at to guard against stale edits. On **409**, fetch the latest ticket and reconcile before retrying.\n\nChanging status, assignee, parent, dates, or milestone creates activity-log entries. **400** covers invalid references, cycles, and workflow violations. **403** means the caller lacks project edit access.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 409, description = "The ticket changed after the supplied version. Fetch the latest ticket before retrying."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_ticket() {}

#[utoipa::path(
    delete,
    path = "/api/v1/tickets/{id}",
    operation_id = "delete_ticket",
    tag = "Tickets and comments",
    summary = "Delete a ticket",
    description = "### When to use\nDelete a ticket. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_ticket() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/{id}/children",
    operation_id = "list_children",
    tag = "Tickets and comments",
    summary = "List child tickets",
    description = "### When to use\nList child tickets. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_children() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/{id}/dependencies",
    operation_id = "list_dependencies",
    tag = "Dependencies and planning",
    summary = "List ticket dependencies",
    description = "### When to use\nList ticket dependencies. Use this endpoint as part of the dependencies and planning workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_dependencies() {}

#[utoipa::path(
    post,
    path = "/api/v1/tickets/{id}/dependencies",
    operation_id = "create_dependency",
    tag = "Dependencies and planning",
    summary = "Create a ticket dependency",
    request_body = DependencyCreateRequest,
    description = "### When to use\nDeclare that the current ticket depends on another active ticket in the same project.\n\n### Request example\n```json\n{\"direction\":\"predecessor\",\"other_ticket_id\":17}\n```\n\nThe server rejects self-dependencies and graph cycles with **400**. Remove the conflicting edge before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 201, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_dependency() {}

#[utoipa::path(
    delete,
    path = "/api/v1/tickets/{id}/dependencies/{dependency_id}",
    operation_id = "delete_dependency",
    tag = "Dependencies and planning",
    summary = "Delete a ticket dependency",
    description = "### When to use\nRemove an active dependency linked to the ticket in the path. Both tickets must be active members of the same project. The removed entry stays in the ticket timeline.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" ), ("dependency_id" = i64, Path, description = "Resource identifier: dependency_id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The active dependency does not belong to the path ticket or is unavailable in its project."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_dependency() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/{id}/comments",
    operation_id = "list_comments",
    tag = "Tickets and comments",
    summary = "List ticket comments",
    description = "### When to use\nList ticket comments. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_comments() {}

#[utoipa::path(
    post,
    path = "/api/v1/tickets/{id}/comments",
    operation_id = "create_comment",
    tag = "Tickets and comments",
    summary = "Add a ticket comment",
    request_body = CommentCreateRequest,
    description = "### When to use\nAppend a comment to a ticket and notify mentioned users.\n\n### Request example\n```json\n{\"content\":\"The migration is ready for review. @alice please check it.\"}\n```\n\n**404** means the ticket was deleted or is outside the caller's access.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_comment() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/{id}/activity",
    operation_id = "list_ticket_activity",
    tag = "Tickets and comments",
    summary = "List ticket activity",
    description = "### When to use\nList ticket activity. Use this endpoint as part of the tickets and comments workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_ticket_activity() {}

#[utoipa::path(
    get,
    path = "/api/v1/tickets/{id}/actual-duration",
    operation_id = "get_actual_duration",
    tag = "Work sessions",
    summary = "Get actual ticket duration",
    description = "### When to use\nGet actual ticket duration. Use this endpoint as part of the work sessions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_actual_duration() {}

#[utoipa::path(
    post,
    path = "/api/v1/work-sessions/start",
    operation_id = "start_work_session",
    tag = "Work sessions",
    summary = "Start a work session",
    request_body = WorkSessionRequest,
    description = "### When to use\nStart the current user's work timer. A user can have only one open session. Calling this while a session is open returns **400**; end the existing session first.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn start_work_session() {}

#[utoipa::path(
    post,
    path = "/api/v1/work-sessions/end",
    operation_id = "end_work_session",
    tag = "Work sessions",
    summary = "End a work session",
    description = "### When to use\nStop the current user's open work timer. Calling this without an open session returns **400**.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn end_work_session() {}

#[utoipa::path(
    get,
    path = "/api/v1/work-sessions/current",
    operation_id = "get_current_work_session",
    tag = "Work sessions",
    summary = "Get the open work session",
    description = "### When to use\nGet the open work session. Use this endpoint as part of the work sessions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_current_work_session() {}

#[utoipa::path(
    get,
    path = "/api/v1/work-sessions/today",
    operation_id = "get_today_work",
    tag = "Work sessions",
    summary = "Get today's work summary",
    description = "### When to use\nGet today's work summary. Use this endpoint as part of the work sessions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_today_work() {}

#[utoipa::path(
    get,
    path = "/api/v1/work-sessions/day",
    operation_id = "get_day_work",
    tag = "Work sessions",
    summary = "Get work for a day",
    description = "### When to use\nGet work for a day. Use this endpoint as part of the work sessions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_day_work() {}

#[utoipa::path(
    get,
    path = "/api/v1/work-sessions/monthly",
    operation_id = "get_monthly_work",
    tag = "Work sessions",
    summary = "Get monthly work summaries",
    description = "### When to use\nGet monthly work summaries. Use this endpoint as part of the work sessions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_monthly_work() {}

#[utoipa::path(
    patch,
    path = "/api/v1/work-sessions/{id}",
    operation_id = "update_work_session",
    tag = "Work sessions",
    summary = "Correct a work session",
    description = "### When to use\nCorrect a work session. Use this endpoint as part of the work sessions workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_work_session() {}

#[utoipa::path(
    get,
    path = "/api/v1/daily-reports",
    operation_id = "list_daily_reports",
    tag = "Daily reports",
    summary = "List daily reports",
    description = "### When to use\nList daily reports. Use this endpoint as part of the daily reports workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_daily_reports() {}

#[utoipa::path(
    post,
    path = "/api/v1/daily-reports",
    operation_id = "submit_daily_report",
    tag = "Daily reports",
    summary = "Submit a daily report",
    request_body = DailyReportRequest,
    description = "### When to use\nCreate or replace the current user's report for a date. The body contains `date`, `good`, `better`, `try`, and `problem` fields.\n\nA report in a closed month cannot be changed. Reopen the month or ask an approver to reject the decision first.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn submit_daily_report() {}

#[utoipa::path(
    get,
    path = "/api/v1/daily-reports/today/tickets",
    operation_id = "get_today_report_tickets",
    tag = "Daily reports",
    summary = "Get tickets touched today",
    description = "### When to use\nGet tickets touched today. Use this endpoint as part of the daily reports workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_today_report_tickets() {}

#[utoipa::path(
    get,
    path = "/api/v1/daily-reports/{date}",
    operation_id = "get_daily_report",
    tag = "Daily reports",
    summary = "Get a daily report",
    description = "### When to use\nGet a daily report. Use this endpoint as part of the daily reports workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("date" = i64, Path, description = "Resource identifier: date" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_daily_report() {}

#[utoipa::path(
    get,
    path = "/api/v1/notifications",
    operation_id = "list_notifications",
    tag = "Notifications",
    summary = "List notifications",
    description = "### When to use\nList notifications. Use this endpoint as part of the notifications workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_notifications() {}

#[utoipa::path(
    get,
    path = "/api/v1/notifications/unread-count",
    operation_id = "get_unread_count",
    tag = "Notifications",
    summary = "Get the unread count",
    description = "### When to use\nGet the unread count. Use this endpoint as part of the notifications workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_unread_count() {}

#[utoipa::path(
    post,
    path = "/api/v1/notifications/read-all",
    operation_id = "mark_all_notifications_read",
    tag = "Notifications",
    summary = "Mark all notifications read",
    description = "### When to use\nMark all notifications read. Use this endpoint as part of the notifications workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn mark_all_notifications_read() {}

#[utoipa::path(
    post,
    path = "/api/v1/notifications/{id}/read",
    operation_id = "mark_notification_read",
    tag = "Notifications",
    summary = "Mark a notification read",
    description = "### When to use\nMark a notification read. Use this endpoint as part of the notifications workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn mark_notification_read() {}

#[utoipa::path(
    get,
    path = "/api/v1/approvals/subordinates",
    operation_id = "list_subordinates",
    tag = "Approvals and month closing",
    summary = "List subordinates",
    description = "### When to use\nList subordinates. Use this endpoint as part of the approvals and month closing workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_subordinates() {}

#[utoipa::path(
    get,
    path = "/api/v1/approvals/subordinates/{user_id}/monthly",
    operation_id = "get_monthly_review",
    tag = "Approvals and month closing",
    summary = "Get a monthly review",
    description = "### When to use\nGet a monthly review. Use this endpoint as part of the approvals and month closing workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("user_id" = i64, Path, description = "Resource identifier: user_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_monthly_review() {}

#[utoipa::path(
    get,
    path = "/api/v1/approvals/subordinates/{user_id}/day/{date}",
    operation_id = "get_day_review",
    tag = "Approvals and month closing",
    summary = "Get a daily review",
    description = "### When to use\nGet a daily review. Use this endpoint as part of the approvals and month closing workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("user_id" = i64, Path, description = "Resource identifier: user_id" ), ("date" = i64, Path, description = "Resource identifier: date" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_day_review() {}

#[utoipa::path(
    post,
    path = "/api/v1/approvals/subordinates/{user_id}/day/{date}/decide",
    operation_id = "decide_daily_report",
    tag = "Approvals and month closing",
    summary = "Approve or reject a daily report",
    request_body = MonthDecisionRequest,
    description = "### When to use\nApprove or reject a subordinate's daily report. The caller must be assigned as that user's approver.\n\n### Request example\n```json\n{\"decision\":\"approved\",\"comment\":\"Reviewed\"}\n```\n\n**403** means the caller is not an approver for the target user. **400** means the report is locked or the decision is invalid.", params(("user_id" = i64, Path, description = "Resource identifier: user_id" ), ("date" = i64, Path, description = "Resource identifier: date" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn decide_daily_report() {}

#[utoipa::path(
    post,
    path = "/api/v1/approvals/subordinates/{user_id}/month/{year}/{month}/close",
    operation_id = "close_month",
    tag = "Approvals and month closing",
    summary = "Close a month",
    request_body = MonthDecisionRequest,
    description = "### When to use\nLock a subordinate's month after every day with work sessions has been approved. Once closed, work sessions and reports cannot be edited until the month is reopened.", params(("user_id" = i64, Path, description = "Resource identifier: user_id" ), ("year" = i64, Path, description = "Resource identifier: year" ), ("month" = i64, Path, description = "Resource identifier: month" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn close_month() {}

#[utoipa::path(
    post,
    path = "/api/v1/approvals/subordinates/{user_id}/month/{year}/{month}/reopen",
    operation_id = "reopen_month",
    tag = "Approvals and month closing",
    summary = "Reopen a month",
    description = "### When to use\nReopen a month. Use this endpoint as part of the approvals and month closing workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("user_id" = i64, Path, description = "Resource identifier: user_id" ), ("year" = i64, Path, description = "Resource identifier: year" ), ("month" = i64, Path, description = "Resource identifier: month" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn reopen_month() {}

#[utoipa::path(
    get,
    path = "/api/v1/settings",
    operation_id = "get_settings",
    tag = "Administration and backup",
    summary = "Get system settings",
    description = "### When to use\nGet system settings. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_settings() {}

#[utoipa::path(
    patch,
    path = "/api/v1/settings",
    operation_id = "update_settings",
    tag = "Administration and backup",
    summary = "Update system settings",
    description = "### When to use\nChange global settings such as maintenance mode, default timezone, or KaTeX rendering. This is administrator-only.\n\nWhen maintenance mode is enabled, normal write endpoints return **503**. Reads remain available; disable maintenance mode or wait for the operation to finish.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_settings() {}

#[utoipa::path(
    get,
    path = "/api/v1/workflows",
    operation_id = "list_workflows",
    tag = "Administration and backup",
    summary = "List workflows",
    description = "### When to use\nList workflows. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn list_workflows() {}

#[utoipa::path(
    post,
    path = "/api/v1/workflows",
    operation_id = "create_workflow",
    tag = "Administration and backup",
    summary = "Create a workflow",
    description = "### When to use\nCreate a workflow. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.",
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_workflow() {}

#[utoipa::path(
    get,
    path = "/api/v1/workflows/{id}",
    operation_id = "get_workflow",
    tag = "Administration and backup",
    summary = "Get a workflow",
    description = "### When to use\nGet a workflow. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn get_workflow() {}

#[utoipa::path(
    patch,
    path = "/api/v1/workflows/{id}",
    operation_id = "update_workflow",
    tag = "Administration and backup",
    summary = "Update a workflow",
    description = "### When to use\nUpdate a workflow. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_workflow() {}

#[utoipa::path(
    delete,
    path = "/api/v1/workflows/{id}",
    operation_id = "delete_workflow",
    tag = "Administration and backup",
    summary = "Delete a workflow",
    description = "### When to use\nDelete a workflow. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_workflow() {}

#[utoipa::path(
    post,
    path = "/api/v1/workflows/{id}/statuses",
    operation_id = "create_status",
    tag = "Administration and backup",
    summary = "Create a workflow status",
    description = "### When to use\nCreate a workflow status. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_status() {}

#[utoipa::path(
    patch,
    path = "/api/v1/workflows/{id}/statuses/{status_id}",
    operation_id = "update_status",
    tag = "Administration and backup",
    summary = "Update a workflow status",
    description = "### When to use\nUpdate a workflow status. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" ), ("status_id" = i64, Path, description = "Resource identifier: status_id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn update_status() {}

#[utoipa::path(
    delete,
    path = "/api/v1/workflows/{id}/statuses/{status_id}",
    operation_id = "delete_status",
    tag = "Administration and backup",
    summary = "Delete a workflow status",
    description = "### When to use\nDelete a workflow status. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" ), ("status_id" = i64, Path, description = "Resource identifier: status_id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_status() {}

#[utoipa::path(
    post,
    path = "/api/v1/workflows/{id}/transitions",
    operation_id = "create_transition",
    tag = "Administration and backup",
    summary = "Create a workflow transition",
    description = "### When to use\nCreate a workflow transition. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" )),
    responses((status = 200, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn create_transition() {}

#[utoipa::path(
    delete,
    path = "/api/v1/workflows/{id}/transitions/{transition_id}",
    operation_id = "delete_transition",
    tag = "Administration and backup",
    summary = "Delete a workflow transition",
    description = "### When to use\nDelete a workflow transition. Use this endpoint as part of the administration and backup workflow.\n\n### Errors\nThe common 400, 401, 403, 404, and 503 responses explain how to recover. Check the `error` field before retrying.", params(("id" = i64, Path, description = "Resource identifier: id" ), ("transition_id" = i64, Path, description = "Resource identifier: transition_id" )),
    responses((status = 204, description = "Successful response. See the response schema and example above."), (status = 400, description = "Invalid input or invalid state. Correct the request using the error message."), (status = 401, description = "Authentication is missing or expired. Log in again."), (status = 403, description = "The caller lacks permission or the CSRF token is missing or invalid."), (status = 404, description = "The resource does not exist or is not visible to the caller."), (status = 503, description = "Maintenance mode is active. Stop mutations and retry later."))
)]
fn delete_transition() {}
