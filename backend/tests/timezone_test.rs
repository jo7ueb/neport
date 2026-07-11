mod common;
use common::{admin_client, base_url, unauthenticated_client, TestUser};
use serde_json::{json, Value};

#[tokio::test]
async fn new_users_default_to_the_system_timezone() {
    let user = TestUser::create().await;

    // Read the current system default directly rather than assuming it's
    // still the out-of-the-box "Asia/Tokyo" - settings_test may have
    // changed it, and this is a single shared row across all tests.
    let admin = admin_client().await;
    let settings_resp = admin.get(format!("{}/api/v1/settings", base_url())).send().await.unwrap();
    let settings: Value = settings_resp.json().await.unwrap();
    let system_default = settings["default_timezone"].as_str().unwrap().to_string();

    let resp = user
        .client
        .get(format!("{}/api/v1/auth/timezone", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["timezone"].is_null(), "a fresh user should have no override");
    assert_eq!(body["effective_timezone"], system_default, "a fresh user should follow the system default");
}

#[tokio::test]
async fn user_can_set_and_clear_their_own_timezone_override() {
    let user = TestUser::create().await;
    let admin = admin_client().await;
    let settings_resp = admin.get(format!("{}/api/v1/settings", base_url())).send().await.unwrap();
    let settings: Value = settings_resp.json().await.unwrap();
    let system_default = settings["default_timezone"].as_str().unwrap().to_string();

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/timezone", base_url()))
        .json(&json!({ "timezone": "America/New_York" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["timezone"], "America/New_York");
    assert_eq!(body["effective_timezone"], "America/New_York");

    // Persists across sessions.
    let other_session = user.login_again().await;
    let resp = other_session
        .get(format!("{}/api/v1/auth/timezone", base_url()))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["timezone"], "America/New_York");

    // Clearing (null) reverts to the system default.
    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/timezone", base_url()))
        .json(&json!({ "timezone": null }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["timezone"].is_null());
    assert_eq!(body["effective_timezone"], system_default);
}

#[tokio::test]
async fn invalid_timezone_name_is_rejected() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/timezone", base_url()))
        .json(&json!({ "timezone": "Not/AZone" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn unauthenticated_request_cannot_read_or_change_timezone() {
    let client = unauthenticated_client();

    let resp = client.get(format!("{}/api/v1/auth/timezone", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 401);

    let resp = client
        .patch(format!("{}/api/v1/auth/timezone", base_url()))
        .json(&json!({ "timezone": "UTC" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn admin_can_change_the_system_default_timezone_and_it_is_rejected_when_invalid() {
    let admin = admin_client().await;

    // Read current settings first so we can restore them (this is a
    // single shared system-wide row, so tests must not leak state).
    let resp = admin.get(format!("{}/api/v1/settings", base_url())).send().await.unwrap();
    let original: Value = resp.json().await.unwrap();
    let original_katex = original["katex_enabled"].as_bool().unwrap();
    let original_tz = original["default_timezone"].as_str().unwrap().to_string();
    let original_maintenance = original["maintenance_mode"].as_bool().unwrap();

    let resp = admin
        .patch(format!("{}/api/v1/settings", base_url()))
        .json(&json!({ "katex_enabled": original_katex, "default_timezone": "UTC", "maintenance_mode": original_maintenance }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: Value = resp.json().await.unwrap();
    assert_eq!(updated["default_timezone"], "UTC");

    let resp = admin
        .patch(format!("{}/api/v1/settings", base_url()))
        .json(&json!({ "katex_enabled": original_katex, "default_timezone": "Not/AZone", "maintenance_mode": original_maintenance }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Restore.
    let resp = admin
        .patch(format!("{}/api/v1/settings", base_url()))
        .json(&json!({ "katex_enabled": original_katex, "default_timezone": original_tz, "maintenance_mode": original_maintenance }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn general_user_cannot_change_system_settings() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/settings", base_url()))
        .json(&json!({ "katex_enabled": true, "default_timezone": "Asia/Tokyo" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}
