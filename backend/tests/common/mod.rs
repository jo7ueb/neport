use reqwest::{cookie::Jar, Client};
use serde_json::{json, Value};
use std::sync::Arc;

/// Base URL of the running backend. Override with TEST_BASE_URL when running
/// against a container by service name (e.g. http://backend:3000) instead of
/// localhost.
pub fn base_url() -> String {
    std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

/// "Now", in the system's default timezone (Asia/Tokyo - see the
/// `timezones` migration). The backend buckets work sessions/daily
/// reports/ticket activity into calendar days using each user's effective
/// timezone (backend/src/time.rs), not the UTC calendar date, so tests that
/// need "today"'s date string for a URL or request body must compute it the
/// same way - otherwise they flake specifically during the UTC-vs-JST
/// mismatch window (00:00-08:59 JST, when the UTC and JST calendar dates
/// differ).
pub fn now_jst() -> chrono::DateTime<chrono_tz::Tz> {
    chrono::Utc::now().with_timezone(&chrono_tz::Asia::Tokyo)
}

/// Generates a collision-free string for emails/project names so tests can
/// run in parallel against the same shared database without UNIQUE conflicts.
pub fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", prefix, nanos)
}

pub fn unauthenticated_client() -> Client {
    Client::builder().cookie_store(true).build().unwrap()
}

/// Credentials of the initial admin bootstrapped by the backend at startup
/// from ADMIN_EMAIL/ADMIN_PASSWORD (see backend/src/main.rs). The test suite
/// relies on this account existing to create further test users, since user
/// creation now requires an admin session.
pub fn admin_email() -> String {
    std::env::var("TEST_ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".to_string())
}

pub fn admin_password() -> String {
    std::env::var("TEST_ADMIN_PASSWORD").unwrap_or_else(|_| "change_me_now_12345".to_string())
}

/// Logs in and returns a client that carries both the session cookie and the
/// X-CSRF-Token header on every subsequent request, mirroring how the real
/// frontend (frontend/src/api/client.ts) behaves. State-changing endpoints
/// validate this header against the session's csrf_token, so any client used
/// to call POST/PATCH/DELETE endpoints in tests must go through this helper.
pub async fn login_with_csrf(email: &str, password: &str) -> Client {
    let jar = Arc::new(Jar::default());
    let bootstrap = Client::builder().cookie_provider(jar.clone()).build().unwrap();
    let resp = bootstrap
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .expect("login request failed");
    assert_eq!(resp.status(), 200, "login should succeed");
    assert!(
        resp.headers().get(reqwest::header::SET_COOKIE).is_some(),
        "login response must set a session cookie"
    );
    let body: Value = resp.json().await.unwrap();
    let csrf_token = body["csrf_token"]
        .as_str()
        .expect("login response must include csrf_token")
        .to_string();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("X-CSRF-Token", csrf_token.parse().unwrap());
    Client::builder()
        .cookie_provider(jar)
        .default_headers(headers)
        .build()
        .unwrap()
}

/// Logs in as the bootstrapped admin and returns a ready-to-use client.
pub async fn admin_client() -> Client {
    login_with_csrf(&admin_email(), &admin_password()).await
}

/// A logged-in user with its own cookie-jar client, ready to call
/// authenticated endpoints.
pub struct TestUser {
    pub client: Client,
    pub id: i64,
    pub email: String,
    pub password: String,
}

impl TestUser {
    async fn create_with_role(role: &str) -> Self {
        let email = format!("{}@example.com", unique("user"));
        let password = "TestPassword123!".to_string();

        let admin = admin_client().await;
        let resp = admin
            .post(format!("{}/api/v1/users", base_url()))
            .json(&json!({
                "email": email,
                "password": password,
                "display_name": "Test User",
                "role": role,
            }))
            .send()
            .await
            .expect("create user request failed");
        assert_eq!(resp.status(), 201, "user creation should succeed");
        let body: Value = resp.json().await.unwrap();
        let id = body["id"].as_i64().expect("user id should be an integer");

        let client = login_with_csrf(&email, &password).await;

        TestUser { client, id, email, password }
    }

    /// Opens an additional, independent session for this same user (its own
    /// cookie jar), useful for proving that a password change/reset
    /// invalidates sessions other than the one that made the change.
    pub async fn login_again(&self) -> Client {
        login_with_csrf(&self.email, &self.password).await
    }

    pub async fn create() -> Self {
        Self::create_with_role("general").await
    }

    pub async fn create_admin() -> Self {
        Self::create_with_role("admin").await
    }

    /// Creates a project (admin-only endpoint, so this always goes through
    /// an admin session regardless of `self`'s own role), passing `self` as
    /// the sole initial manager - matching the pre-restriction behavior
    /// where whoever created a project automatically became its one and
    /// only first manager. Some tests (e.g. "the project's only manager
    /// must not be removable") depend on there being exactly one manager
    /// afterward.
    pub async fn create_project(&self, name: &str) -> Value {
        let admin = admin_client().await;
        let resp = admin
            .post(format!("{}/api/v1/projects", base_url()))
            .json(&json!({
                "name": name,
                "description": "test project",
                "members": [{ "user_id": self.id, "role": "manager" }],
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "project creation should succeed");
        resp.json().await.unwrap()
    }

    pub async fn create_ticket(
        &self,
        project_id: i64,
        parent_ticket_id: Option<i64>,
        title: &str,
    ) -> Value {
        let resp = self
            .client
            .post(format!("{}/api/v1/tickets", base_url()))
            .json(&json!({
                "project_id": project_id,
                "parent_ticket_id": parent_ticket_id,
                "assignee_id": null,
                "title": title,
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "ticket creation should succeed");
        resp.json().await.unwrap()
    }

    /// Grants this user the approver flag and assigns them as `subject`'s
    /// approver, via the admin-only endpoints. Used to set up the approval
    /// workflow's permission graph before exercising it.
    pub async fn become_approver_of(&self, subject: &TestUser) {
        let admin = admin_client().await;
        let resp = admin
            .patch(format!("{}/api/v1/users/{}/is-approver", base_url(), self.id))
            .json(&json!({ "is_approver": true }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "granting the approver flag should succeed");

        let resp = admin
            .post(format!("{}/api/v1/users/{}/approvers", base_url(), subject.id))
            .json(&json!({ "approver_id": self.id }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "assigning the approver should succeed");
    }

    /// Looks up the id of the status with the given `key` (e.g. "open",
    /// "done") in the given project's workflow - every project defaults to
    /// the seeded "シンプル" workflow, which has these four keys, unless created
    /// with an explicit custom workflow_id.
    pub async fn status_id(&self, project_id: i64, key: &str) -> i64 {
        let resp = self
            .client
            .get(format!("{}/api/v1/projects/{}/workflow", base_url(), project_id))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "fetching the project's workflow should succeed");
        let detail: Value = resp.json().await.unwrap();
        detail["statuses"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["key"] == key)
            .unwrap_or_else(|| panic!("no status with key '{}' in this workflow", key))["id"]
            .as_i64()
            .unwrap()
    }
}

pub fn as_i64(v: &Value, field: &str) -> i64 {
    v[field]
        .as_i64()
        .unwrap_or_else(|| panic!("expected field '{}' to be an integer in {:?}", field, v))
}
