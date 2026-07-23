mod common;
use common::{admin_client, base_url, unauthenticated_client, unique, TestUser};
use serde_json::{json, Value};

#[tokio::test]
async fn create_user_and_login_issues_session_cookie_and_csrf_token() {
    let user = TestUser::create().await;
    assert!(user.id > 0);
}

#[tokio::test]
async fn login_with_wrong_password_is_rejected() {
    let user = TestUser::create().await;
    let client = unauthenticated_client();

    let resp = client
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": user.email, "password": "wrong-password" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn login_with_unknown_email_is_rejected() {
    let client = unauthenticated_client();

    let resp = client
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({
            "email": format!("{}@example.com", unique("nobody")),
            "password": "irrelevant"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn unauthenticated_requests_to_protected_endpoints_are_rejected() {
    let client = unauthenticated_client();

    let resp = client
        .get(format!("{}/api/v1/projects", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn suspended_user_is_forbidden_after_session_established() {
    let admin = TestUser::create_admin().await;
    let user = TestUser::create().await;

    // Have an admin flip the user's status and verify the AuthUser extractor
    // rejects subsequent requests from the now-suspended session.
    let resp = admin
        .client
        .post(format!("{}/api/v1/users/{}/suspend", base_url(), user.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/projects", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn successful_login_is_recorded_in_the_audit_log_with_the_client_ip() {
    let user = TestUser::create().await;
    let client = unauthenticated_client();

    let resp = client
        .post(format!("{}/api/v1/auth/login", base_url()))
        .header("X-Real-IP", "203.0.113.9")
        .json(&json!({ "email": user.email, "password": user.password }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let admin = admin_client().await;
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    let entry = entries
        .iter()
        .find(|e| e["action"] == "login" && e["ip_address"] == "203.0.113.9")
        .expect("a successful login must be recorded in the audit log with the given IP");
    assert_eq!(entry["actor_id"], user.id);
}

#[tokio::test]
async fn wrong_password_login_against_a_real_account_is_recorded_as_login_failed() {
    let user = TestUser::create().await;
    let client = unauthenticated_client();

    let resp = client
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": user.email, "password": "wrong-password" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let admin = admin_client().await;
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    let entry = entries
        .iter()
        .find(|e| e["action"] == "login_failed")
        .expect("a wrong-password attempt against a real account must be recorded as login_failed");
    assert_eq!(entry["actor_id"], user.id);
}

#[tokio::test]
async fn logout_is_recorded_in_the_audit_log() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/auth/logout", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let admin = admin_client().await;
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    let entry = entries
        .iter()
        .find(|e| e["action"] == "logout")
        .expect("logging out must be recorded in the audit log");
    assert_eq!(entry["actor_id"], user.id);
}
