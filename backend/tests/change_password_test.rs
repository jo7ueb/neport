mod common;
use common::{admin_client, base_url, unauthenticated_client, TestUser};
use serde_json::{json, Value};

#[tokio::test]
async fn user_can_change_their_own_password() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/password", base_url()))
        .json(&json!({ "current_password": "TestPassword123!", "new_password": "NewPassword456!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let unauth = unauthenticated_client();

    let resp = unauth
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": user.email, "password": "TestPassword123!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "old password must no longer work");

    let resp = unauth
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": user.email, "password": "NewPassword456!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "new password must work");
}

#[tokio::test]
async fn changing_password_with_wrong_current_password_is_rejected() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/password", base_url()))
        .json(&json!({ "current_password": "totally-wrong", "new_password": "NewPassword456!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn changing_password_to_something_too_short_is_rejected() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/password", base_url()))
        .json(&json!({ "current_password": "TestPassword123!", "new_password": "Ab1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn changing_password_without_a_digit_is_rejected() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/password", base_url()))
        .json(&json!({ "current_password": "TestPassword123!", "new_password": "onlylettershere" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn changing_password_invalidates_other_sessions_but_keeps_the_current_one() {
    let user = TestUser::create().await;

    // A second, independent session for the same user (e.g. another device).
    let other_session = user.login_again().await;
    let resp = other_session
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "the second session must start out valid");

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/password", base_url()))
        .json(&json!({ "current_password": "TestPassword123!", "new_password": "NewPassword456!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = other_session
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "the other session must be invalidated by the password change");

    let resp = user
        .client
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "the session that made the change must remain valid");
}

#[tokio::test]
async fn self_service_password_change_is_recorded_distinctly_from_an_admin_reset() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/password", base_url()))
        .json(&json!({ "current_password": "TestPassword123!", "new_password": "NewPassword456!" }))
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
        .find(|e| e["action"] == "password_changed")
        .expect("a self-service password change must be recorded as password_changed");
    assert_eq!(entry["actor_id"], user.id);
    assert!(
        !entries.iter().any(|e| e["action"] == "user_password_reset"),
        "a self-service change must not be recorded as an admin-triggered reset"
    );
}
