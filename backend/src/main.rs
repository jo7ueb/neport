use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber;

mod access;
mod activity_log;
mod approvals;
mod attachments;
mod backup;
mod audit;
mod auth;
mod config;
mod daily_reports;
mod error;
mod gantt;
mod mentions;
mod milestones;
mod notifications;
mod pagination;
mod project_members;
mod projects;
mod settings;
mod state;
mod storage;
mod tickets;
mod time;
mod users;
mod work_sessions;
mod workflows;

use config::Config;
use error::{AppError, Result};
use state::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    // Load configuration
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // Create database pool
    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            std::process::exit(1);
        }
    };

    // Bootstrap initial admin user (only when the users table is empty)
    if let (Some(email), Some(password), Some(display_name)) = (
        &config.admin_email,
        &config.admin_password,
        &config.admin_display_name,
    ) {
        match users::repo::count_users(&pool).await {
            Ok(0) => match auth::hash_password(password) {
                Ok(hash) => {
                    match users::repo::create_user(
                        &pool,
                        email,
                        &hash,
                        display_name,
                        auth::UserRole::Admin,
                        None,
                    )
                    .await
                    {
                        Ok(_) => tracing::info!("bootstrapped initial admin user: {}", email),
                        Err(e) => tracing::error!("failed to bootstrap initial admin user: {:?}", e),
                    }
                }
                Err(e) => tracing::error!("failed to hash initial admin password: {:?}", e),
            },
            Ok(_) => {}
            Err(e) => tracing::error!("failed to count users for admin bootstrap: {:?}", e),
        }
    }

    let storage = match storage::build(&config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize attachment storage: {}", e);
            std::process::exit(1);
        }
    };

    // Create app state
    let state = Arc::new(AppState {
        db: pool,
        session_cookie_name: config.session_cookie_name.clone(),
        session_ttl_seconds: config.session_ttl_seconds,
        storage,
        database_url: config.database_url.clone(),
    });

    // Rate-limit login attempts per source IP to blunt brute-force guessing.
    // nginx sets X-Forwarded-For/X-Real-IP (see nginx.conf), so SmartIpKeyExtractor
    // reads the real client IP instead of always seeing nginx's own address.
    let governor_conf = build_login_governor_config(
        config.login_rate_limit_per_second,
        config.login_rate_limit_burst,
    );
    {
        let governor_conf = governor_conf.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                governor_conf.limiter().retain_recent();
            }
        });
    }

    // Periodically purge expired session rows so the table doesn't grow
    // unbounded (nothing else ever deletes them once expires_at passes).
    {
        let db = state.db.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match auth::session::cleanup_expired_sessions(&db).await {
                    Ok(n) if n > 0 => tracing::info!("cleaned up {} expired sessions", n),
                    Ok(_) => {}
                    Err(e) => tracing::error!("session cleanup failed: {:?}", e),
                }
            }
        });
    }

    // Periodically notify assignees of tickets that have crossed their due
    // date (due_date has no per-ticket timezone, so this compares against
    // the server's own date - acceptable given ticket dates are already
    // plain, timezone-less DATE columns elsewhere in this codebase). The
    // notifications_ticket_overdue_once partial unique index keeps repeated
    // runs from re-notifying the same ticket.
    {
        let db = state.db.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match notifications::repo::notify_overdue_tickets(&db).await {
                    Ok(n) if n > 0 => tracing::info!("sent {} overdue-ticket notifications", n),
                    Ok(_) => {}
                    Err(e) => tracing::error!("overdue-ticket notification sweep failed: {:?}", e),
                }
            }
        });
    }

    let login_router = Router::new()
        .route("/login", post(auth_login))
        .layer(GovernorLayer { config: governor_conf });

    let auth_router = Router::new()
        .merge(login_router)
        .route("/logout", post(auth_logout))
        .route("/me", get(auth_me))
        .route("/password", axum::routing::patch(change_password))
        .route("/locale", axum::routing::patch(update_locale))
        .route("/timezone", get(get_timezone).patch(update_timezone))
        .route("/profile", axum::routing::patch(update_profile))
        .route("/avatar", post(upload_avatar).delete(delete_own_avatar));

    let users_router = Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/search", get(search_users))
        .route("/:id/avatar", get(get_user_avatar))
        .route("/:id/suspend", post(suspend_user))
        .route("/:id/resume", post(resume_user))
        .route("/:id/reset-password", post(reset_user_password))
        .route("/:id/role", axum::routing::patch(update_user_role))
        .route("/:id/activity", get(user_activity))
        .route("/:id/is-approver", axum::routing::patch(update_is_approver))
        .route("/:id/approvers", get(list_approvers).post(add_approver))
        .route("/:id/approvers/:approver_id", delete(remove_approver));

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1/attachments", attachments::router())
        .nest("/api/v1/backup", backup::router())
        .nest("/api/v1/audit-log", audit::router())
        .nest("/api/v1/auth", auth_router)
        .nest("/api/v1/users", users_router)
        .nest("/api/v1/projects", projects::router())
        .nest("/api/v1/projects/:project_id/milestones", milestones::router())
        .nest("/api/v1/projects/:project_id/gantt", gantt::router())
        .nest("/api/v1/projects/:project_id/members", project_members::router())
        .nest("/api/v1/tickets", tickets::router())
        .nest("/api/v1/work-sessions", work_sessions::router())
        .nest("/api/v1/daily-reports", daily_reports::router())
        .nest("/api/v1/notifications", notifications::router())
        .nest("/api/v1/approvals", approvals::router())
        .nest("/api/v1/settings", settings::router())
        .nest("/api/v1/workflows", workflows::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Run server
    let listener = match tokio::net::TcpListener::bind(&config.bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to address: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("Server listening on {}", config.bind_addr);

    // SmartIpKeyExtractor (used by the login rate limiter) falls back to the
    // TCP peer address when no X-Forwarded-For/X-Real-IP/Forwarded header is
    // present, but only if ConnectInfo is available - otherwise every
    // request fails key extraction with a 500. into_make_service_with_connect_info
    // wires that up.
    let make_service = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    if let Err(e) = axum::serve(listener, make_service)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}

// Cloud Run/docker both stop containers with SIGTERM before SIGKILL, and the
// coverage build (backend/scripts/run-coverage.sh) relies on this returning
// normally on SIGTERM so the LLVM profiling runtime's atexit handler flushes
// *.profraw - a bare `kill`, with no handler installed, skips that entirely.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, starting graceful shutdown");
}

async fn health_check() -> impl IntoResponse {
    Json(json!({"status": "ok"}))
}

fn build_login_governor_config(
    per_second: u64,
    burst: u32,
) -> Arc<tower_governor::governor::GovernorConfig<SmartIpKeyExtractor, governor::middleware::NoOpMiddleware>> {
    Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("rate limit config must be valid (non-zero period/burst)"),
    )
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::{Layer, Service, ServiceExt};

    /// Proves the governor middleware itself rejects requests past its
    /// configured burst, independent of the live integration test server
    /// (which cannot exercise this: every integration test shares one
    /// apparent source IP, so a tight limit there would make unrelated
    /// tests flake instead of proving anything about this mechanism).
    #[tokio::test]
    async fn requests_past_the_burst_are_rejected_with_429() {
        let config = build_login_governor_config(1, 2);
        let inner = tower::service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(axum::response::Response::new(Body::empty()))
        });
        let mut svc = tower_governor::GovernorLayer { config }.layer(inner);

        let mut statuses = Vec::new();
        for _ in 0..3 {
            let req = Request::builder()
                .uri("/login")
                .header("x-forwarded-for", "203.0.113.1")
                .body(Body::empty())
                .unwrap();
            let resp = svc.ready().await.unwrap().call(req).await.unwrap();
            statuses.push(resp.status());
        }

        assert_eq!(statuses[0], StatusCode::OK);
        assert_eq!(statuses[1], StatusCode::OK);
        assert_eq!(statuses[2], StatusCode::TOO_MANY_REQUESTS, "3rd request must exceed the burst of 2");
    }
}

async fn auth_login(
    State(state): State<Arc<AppState>>,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Json(payload): Json<auth::LoginRequest>,
) -> Result<(StatusCode, [(header::HeaderName, String); 1], Json<auth::LoginResponse>)> {
    let user_data = users::repo::get_user_by_email(&state.db, &payload.email)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    if user_data.user.status != auth::UserStatus::Active {
        audit::repo::record(&state.db, user_data.user.id, "login_failed", "user", Some(user_data.user.id), None, None, ip.as_deref()).await?;
        return Err(AppError::InvalidCredentials);
    }

    let password_valid = auth::verify_password(&payload.password, &user_data.password_hash)
        .map_err(|_| AppError::InvalidCredentials)?;

    if !password_valid {
        audit::repo::record(&state.db, user_data.user.id, "login_failed", "user", Some(user_data.user.id), None, None, ip.as_deref()).await?;
        return Err(AppError::InvalidCredentials);
    }

    let session = auth::create_session(&state.db, user_data.user.id, state.session_ttl_seconds)
        .await?;

    audit::repo::record(&state.db, user_data.user.id, "login", "user", Some(user_data.user.id), None, None, ip.as_deref()).await?;

    let cookie = format!(
        "{}={}; Path=/; HttpOnly; Secure; SameSite=Lax",
        state.session_cookie_name, session.id
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(auth::LoginResponse {
            user: user_data.user,
            csrf_token: session.csrf_token,
        }),
    ))
}

async fn auth_logout(
    State(state): State<Arc<AppState>>,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    headers: axum::http::HeaderMap,
) -> Result<(StatusCode, [(header::HeaderName, String); 1], Json<serde_json::Value>)> {
    if let Some(cookie_header) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        if let Some(session_id_str) =
            auth::extractors::parse_cookie(cookie_header, &state.session_cookie_name)
        {
            if let Ok(session_id) = uuid::Uuid::parse_str(&session_id_str) {
                if let Some(sess) = auth::session::get_session(&state.db, session_id).await? {
                    let provided = headers.get("X-CSRF-Token").and_then(|v| v.to_str().ok());
                    if provided != Some(sess.csrf_token.as_str()) {
                        return Err(AppError::InvalidCsrf);
                    }
                    audit::repo::record(&state.db, sess.user_id, "logout", "user", Some(sess.user_id), None, None, ip.as_deref()).await?;
                }
                let _ = auth::session::delete_session(&state.db, session_id).await;
            }
        }
    }

    let clear_cookie = format!(
        "{}=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0",
        state.session_cookie_name
    );

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, clear_cookie)],
        Json(json!({"message": "Logged out"})),
    ))
}

async fn auth_me(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
) -> Result<Json<auth::LoginResponse>> {
    // The frontend only persists the CSRF token in memory (see
    // frontend/src/api/client.ts), so it's lost on every page reload. /me is
    // how the app restores the logged-in session on load, so it must also
    // hand back the current csrf_token or every mutation would 403 after a
    // reload until the user logs out and back in.
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let session_id = auth::extractors::parse_cookie(cookie_header, &state.session_cookie_name)
        .and_then(|v| uuid::Uuid::parse_str(&v).ok())
        .ok_or(AppError::Unauthorized)?;
    let sess = auth::session::get_session(&state.db, session_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(auth::LoginResponse {
        user,
        csrf_token: sess.csrf_token,
    }))
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    email: String,
    password: String,
    display_name: String,
    #[serde(default)]
    role: Option<auth::UserRole>,
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<auth::User>)> {
    auth::password::validate_strength(&payload.password)?;

    let password_hash = auth::hash_password(&payload.password)
        .map_err(|_| AppError::InternalServerError)?;

    let user = users::repo::create_user(
        &state.db,
        &payload.email,
        &password_hash,
        &payload.display_name,
        payload.role.unwrap_or(auth::UserRole::General),
        Some(admin.id),
    )
    .await?;

    audit::repo::record(&state.db, admin.id, "user_created", "user", Some(user.id), None, Some(&user.email), ip.as_deref()).await?;

    Ok((StatusCode::CREATED, Json(user)))
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    _auth: auth::extractors::AuthUser,
    Query(page): Query<pagination::PageQuery>,
) -> Result<Json<pagination::Paginated<auth::User>>> {
    let users = users::repo::list_users(&state.db, page.limit(), page.offset()).await?;
    let total = users::repo::count_listed_users(&state.db).await?;
    Ok(Json(pagination::Paginated::new(users, total, &page)))
}

#[derive(Debug, serde::Deserialize)]
struct SearchUsersQuery {
    q: Option<String>,
}

// Any authenticated user can search, same access level as ticket search -
// this backs picker widgets (project manager/member selects, etc.) where a
// plain <select> of every user becomes unusable once there are more than a
// handful of accounts.
async fn search_users(
    State(state): State<Arc<AppState>>,
    _auth: auth::extractors::AuthUser,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<Vec<auth::User>>> {
    let q = query.q.unwrap_or_default();
    let users = users::repo::search_users(&state.db, &q, 20).await?;
    Ok(Json(users))
}

async fn suspend_user(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    if admin.id == user_id {
        return Err(AppError::BadRequest("cannot suspend your own account".into()));
    }
    users::repo::suspend_user(&state.db, user_id).await?;
    audit::repo::record(&state.db, admin.id, "user_suspended", "user", Some(user_id), None, None, ip.as_deref()).await?;
    Ok(Json(json!({"message": "User suspended"})))
}

async fn resume_user(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    users::repo::resume_user(&state.db, user_id).await?;
    audit::repo::record(&state.db, admin.id, "user_resumed", "user", Some(user_id), None, None, ip.as_deref()).await?;
    Ok(Json(json!({"message": "User resumed"})))
}

#[derive(Debug, Deserialize)]
struct UpdateRoleRequest {
    role: auth::UserRole,
}

async fn update_user_role(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Path(user_id): Path<i64>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<serde_json::Value>> {
    if admin.id == user_id {
        return Err(AppError::BadRequest("cannot change your own role".into()));
    }
    let target = users::repo::get_user_by_id(&state.db, user_id).await?.ok_or(AppError::NotFound)?;
    users::repo::update_role(&state.db, user_id, payload.role).await?;
    audit::repo::record(
        &state.db,
        admin.id,
        "user_role_changed",
        "user",
        Some(user_id),
        Some(&target.role.to_string()),
        Some(&payload.role.to_string()),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(json!({ "message": "Role updated" })))
}

#[derive(Debug, Deserialize)]
struct UpdateIsApproverRequest {
    is_approver: bool,
}

async fn update_is_approver(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Path(user_id): Path<i64>,
    Json(payload): Json<UpdateIsApproverRequest>,
) -> Result<Json<serde_json::Value>> {
    let target = users::repo::get_user_by_id(&state.db, user_id).await?.ok_or(AppError::NotFound)?;
    users::repo::set_is_approver(&state.db, user_id, payload.is_approver).await?;
    audit::repo::record(
        &state.db,
        admin.id,
        "user_approver_flag_changed",
        "user",
        Some(user_id),
        Some(&target.is_approver.to_string()),
        Some(&payload.is_approver.to_string()),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(json!({ "message": "Updated" })))
}

async fn list_approvers(
    State(state): State<Arc<AppState>>,
    _admin: auth::extractors::AdminUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Vec<auth::User>>> {
    let approvers = approvals::repo::list_approvers_for_user(&state.db, user_id).await?;
    Ok(Json(approvers))
}

#[derive(Debug, Deserialize)]
struct AddApproverRequest {
    approver_id: i64,
}

async fn add_approver(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    Path(user_id): Path<i64>,
    Json(payload): Json<AddApproverRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let approver = users::repo::get_user_by_id(&state.db, payload.approver_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !approver.is_approver {
        return Err(AppError::BadRequest(
            "this user does not have the approver flag enabled".to_string(),
        ));
    }
    approvals::repo::add_approver(&state.db, user_id, payload.approver_id, Some(admin.id)).await?;
    Ok((StatusCode::CREATED, Json(json!({ "message": "Added" }))))
}

async fn remove_approver(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Path((user_id, approver_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>> {
    approvals::repo::remove_approver(&state.db, user_id, approver_id).await?;
    audit::repo::record(
        &state.db,
        admin.id,
        "approver_removed",
        "user_approver",
        Some(user_id),
        None,
        Some(&approver_id.to_string()),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(json!({ "message": "Removed" })))
}

async fn reset_user_password(
    State(state): State<Arc<AppState>>,
    auth::extractors::AdminUser(admin): auth::extractors::AdminUser,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let temp_password = auth::password::generate_temp_password();
    let hash = auth::hash_password(&temp_password).map_err(|_| AppError::InternalServerError)?;
    users::repo::update_password_hash(&state.db, user_id, &hash).await?;
    // An admin-triggered reset means the target user no longer controls
    // their old password, so any of their existing sessions could belong to
    // whoever had it - kill all of them.
    auth::session::delete_all_sessions_for_user(&state.db, user_id).await?;
    audit::repo::record(&state.db, admin.id, "user_password_reset", "user", Some(user_id), None, None, ip.as_deref()).await?;
    Ok(Json(json!({ "temporary_password": temp_password })))
}

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    audit::client_ip::ClientIp(ip): audit::client_ip::ClientIp,
    headers: axum::http::HeaderMap,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>> {
    auth::password::validate_strength(&payload.new_password)?;

    let current_hash = users::repo::get_password_hash_by_id(&state.db, user.id)
        .await?
        .ok_or(AppError::InternalServerError)?;

    let valid = auth::verify_password(&payload.current_password, &current_hash)
        .map_err(|_| AppError::InternalServerError)?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    let new_hash = auth::hash_password(&payload.new_password).map_err(|_| AppError::InternalServerError)?;
    users::repo::update_password_hash(&state.db, user.id, &new_hash).await?;

    // Kill any other sessions (e.g. a stolen cookie elsewhere) now that the
    // password has changed, but keep the caller's own session alive so they
    // aren't immediately logged out by the change they just made.
    if let Some(session_id) =
        auth::extractors::session_id_from_headers(&headers, &state.session_cookie_name)
    {
        auth::session::delete_other_sessions_for_user(&state.db, user.id, session_id).await?;
    }

    audit::repo::record(&state.db, user.id, "password_changed", "user", Some(user.id), None, None, ip.as_deref()).await?;

    Ok(Json(json!({ "message": "Password updated" })))
}

#[derive(Debug, Deserialize)]
struct UpdateLocaleRequest {
    locale: String,
}

async fn update_locale(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
    Json(payload): Json<UpdateLocaleRequest>,
) -> Result<Json<serde_json::Value>> {
    if !["ja", "en"].contains(&payload.locale.as_str()) {
        return Err(AppError::BadRequest("invalid locale".into()));
    }
    users::repo::update_locale(&state.db, user.id, &payload.locale).await?;
    Ok(Json(json!({ "message": "Locale updated" })))
}

#[derive(serde::Serialize)]
struct TimezoneResponse {
    // The user's own override, or null if they're following the system
    // default (see PATCH .../auth/timezone to set/clear it).
    timezone: Option<String>,
    // The timezone actually used to bucket this user's timestamps into
    // calendar days right now (their override, or the system default).
    effective_timezone: String,
}

async fn get_timezone(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
) -> Result<Json<TimezoneResponse>> {
    let timezone = users::repo::get_timezone_override(&state.db, user.id).await?;
    let effective_timezone = users::repo::resolve_timezone(&state.db, user.id).await?;
    Ok(Json(TimezoneResponse { timezone, effective_timezone }))
}

#[derive(Debug, Deserialize)]
struct UpdateTimezoneRequest {
    // None/null clears the override and reverts to the system default.
    timezone: Option<String>,
}

async fn update_timezone(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
    Json(payload): Json<UpdateTimezoneRequest>,
) -> Result<Json<TimezoneResponse>> {
    if let Some(tz) = &payload.timezone {
        if !time::is_valid_timezone(tz) {
            return Err(AppError::BadRequest("invalid timezone".into()));
        }
    }
    users::repo::set_timezone(&state.db, user.id, payload.timezone.as_deref()).await?;
    let effective_timezone = users::repo::resolve_timezone(&state.db, user.id).await?;
    Ok(Json(TimezoneResponse { timezone: payload.timezone, effective_timezone }))
}

#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    display_name: String,
}

async fn update_profile(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<auth::User>> {
    let display_name = payload.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("display name cannot be empty".into()));
    }
    users::repo::update_display_name(&state.db, user.id, display_name).await?;
    let updated = users::repo::get_user_by_id(&state.db, user.id)
        .await?
        .ok_or(AppError::InternalServerError)?;
    Ok(Json(updated))
}

const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;
const ALLOWED_AVATAR_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

async fn upload_avatar(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<auth::User>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("invalid upload".into()))?
    {
        if field.name() == Some("file") {
            content_type = field.content_type().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("failed to read uploaded file".into()))?;
            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| AppError::BadRequest("no file uploaded".into()))?;
    let content_type = content_type.ok_or_else(|| AppError::BadRequest("missing file type".into()))?;

    if !ALLOWED_AVATAR_TYPES.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest(
            "unsupported image type (must be PNG, JPEG, WebP, or GIF)".into(),
        ));
    }
    if data.len() > MAX_AVATAR_BYTES {
        return Err(AppError::BadRequest("image too large (max 2MB)".into()));
    }

    users::repo::update_avatar(&state.db, user.id, &data, &content_type).await?;
    let updated = users::repo::get_user_by_id(&state.db, user.id)
        .await?
        .ok_or(AppError::InternalServerError)?;
    Ok(Json(updated))
}

async fn delete_own_avatar(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(user): auth::extractors::AuthUser,
) -> Result<Json<auth::User>> {
    users::repo::delete_avatar(&state.db, user.id).await?;
    let updated = users::repo::get_user_by_id(&state.db, user.id)
        .await?
        .ok_or(AppError::InternalServerError)?;
    Ok(Json(updated))
}

async fn get_user_avatar(
    State(state): State<Arc<AppState>>,
    auth::extractors::AuthUser(_user): auth::extractors::AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse> {
    let (data, content_type) = users::repo::get_avatar(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
        ],
        data,
    ))
}

#[derive(Debug, Deserialize)]
struct UserActivityQuery {
    date: chrono::NaiveDate,
}

async fn user_activity(
    State(state): State<Arc<AppState>>,
    _auth: auth::extractors::AuthUser,
    Path(user_id): Path<i64>,
    Query(query): Query<UserActivityQuery>,
) -> Result<Json<Vec<activity_log::TicketActivity>>> {
    let tz = users::repo::resolve_timezone(&state.db, user_id).await?;
    let activities = activity_log::repo::list_for_user_on_date(&state.db, user_id, query.date, &tz).await?;
    Ok(Json(activities))
}
