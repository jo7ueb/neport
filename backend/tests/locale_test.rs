mod common;
use common::{base_url, unauthenticated_client, TestUser};
use serde_json::{json, Value};

#[tokio::test]
async fn new_users_default_to_japanese_locale() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["user"]["locale"], "ja");
}

#[tokio::test]
async fn user_can_switch_locale_and_it_persists_across_sessions() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/locale", base_url()))
        .json(&json!({ "locale": "en" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let other_session = user.login_again().await;
    let resp = other_session
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["user"]["locale"], "en",
        "locale must be persisted server-side, not just in the current session"
    );

    // Switch back so this doesn't leak state into other tests reusing the user.
    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/locale", base_url()))
        .json(&json!({ "locale": "ja" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn invalid_locale_is_rejected() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .patch(format!("{}/api/v1/auth/locale", base_url()))
        .json(&json!({ "locale": "fr" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn unauthenticated_request_cannot_change_locale() {
    let client = unauthenticated_client();

    let resp = client
        .patch(format!("{}/api/v1/auth/locale", base_url()))
        .json(&json!({ "locale": "en" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}
