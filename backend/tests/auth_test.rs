mod common;
use common::{base_url, unauthenticated_client, unique, TestUser};
use serde_json::json;

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
