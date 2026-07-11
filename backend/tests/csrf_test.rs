mod common;
use common::{base_url, unique, TestUser};
use reqwest::{cookie::Jar, Client};
use serde_json::json;
use std::sync::Arc;

/// Logs in like TestUser does, but deliberately omits the X-CSRF-Token
/// default header so tests can exercise the missing/invalid-token paths.
async fn login_without_csrf_header(email: &str, password: &str) -> Client {
    let jar = Arc::new(Jar::default());
    let client = Client::builder().cookie_provider(jar).build().unwrap();
    let resp = client
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    client
}

#[tokio::test]
async fn mutation_without_csrf_header_is_rejected() {
    let email = format!("{}@example.com", unique("csrf-user"));
    let password = "TestPassword123!";
    let admin = common::admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({ "email": email, "password": password, "display_name": "CSRF Test", "role": "general" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let client = login_without_csrf_header(&email, password).await;

    let resp = client
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({ "name": unique("csrf-project"), "description": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "mutating requests without X-CSRF-Token must be rejected");
}

#[tokio::test]
async fn mutation_with_wrong_csrf_token_is_rejected() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/projects", base_url()))
        .header("X-CSRF-Token", "not-the-real-token")
        .json(&json!({ "name": unique("csrf-project"), "description": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn get_requests_do_not_require_csrf_header() {
    let email = format!("{}@example.com", unique("csrf-get-user"));
    let password = "TestPassword123!";
    let admin = common::admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({ "email": email, "password": password, "display_name": "CSRF Get Test", "role": "general" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let client = login_without_csrf_header(&email, password).await;

    let resp = client
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "GET requests must not require the CSRF header");
}

#[tokio::test]
async fn logout_without_csrf_header_is_rejected() {
    let email = format!("{}@example.com", unique("csrf-logout-user"));
    let password = "TestPassword123!";
    let admin = common::admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({ "email": email, "password": password, "display_name": "CSRF Logout Test", "role": "general" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let client = login_without_csrf_header(&email, password).await;

    let resp = client
        .post(format!("{}/api/v1/auth/logout", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

/// Regression test: the frontend only keeps the CSRF token in memory
/// (frontend/src/api/client.ts), so it's lost on every page reload. /auth/me
/// is how the app restores a session on load, so it must hand back the
/// current csrf_token too - otherwise every mutation would 403 after a
/// reload until the user logged out and back in.
#[tokio::test]
async fn auth_me_returns_a_csrf_token_that_works_for_mutations() {
    let email = format!("{}@example.com", unique("csrf-me-user"));
    let password = "TestPassword123!";
    let admin = common::admin_client().await;
    // Uses project creation as "some mutating endpoint" to prove the token
    // round-trip works - that endpoint is admin-only, so this user must be
    // one too (unrelated to what this test is actually verifying).
    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({ "email": email, "password": password, "display_name": "CSRF Me Test", "role": "admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // A client with the session cookie but no default X-CSRF-Token header,
    // simulating a fresh page load that only has the cookie to work with.
    let client = login_without_csrf_header(&email, password).await;

    let resp = client
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let csrf_token = body["csrf_token"]
        .as_str()
        .expect("/auth/me response must include csrf_token")
        .to_string();

    let resp = client
        .post(format!("{}/api/v1/projects", base_url()))
        .header("X-CSRF-Token", csrf_token)
        .json(&json!({ "name": unique("csrf-me-project"), "description": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "the csrf_token from /auth/me must be usable for subsequent mutations");
}
