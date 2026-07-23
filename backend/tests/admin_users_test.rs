mod common;
use common::{admin_client, base_url, unauthenticated_client, unique, TestUser};
use serde_json::{json, Value};

#[tokio::test]
async fn general_user_cannot_create_users() {
    let general = TestUser::create().await;

    let resp = general
        .client
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({
            "email": format!("{}@example.com", unique("blocked")),
            "password": "TestPassword123!",
            "display_name": "Blocked User",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "non-admin must not be able to create users");
}

#[tokio::test]
async fn unauthenticated_request_cannot_create_users() {
    let client = unauthenticated_client();

    let resp = client
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({
            "email": format!("{}@example.com", unique("blocked")),
            "password": "TestPassword123!",
            "display_name": "Blocked User",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "unauthenticated requests must be rejected before role is even checked");
}

#[tokio::test]
async fn creating_a_user_with_a_weak_password_is_rejected() {
    let admin = admin_client().await;

    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({
            "email": format!("{}@example.com", unique("weakpw")),
            "password": "short1",
            "display_name": "Weak Password User",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn admin_can_create_users_with_a_chosen_role() {
    let admin = admin_client().await;

    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({
            "email": format!("{}@example.com", unique("newadmin")),
            "password": "TestPassword123!",
            "display_name": "New Admin",
            "role": "admin",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["role"], "admin");
    let new_user_id = body["id"].as_i64().unwrap();

    let admin_id = admin
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["user"]["id"]
        .as_i64()
        .unwrap();
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), admin_id))
        .send()
        .await
        .unwrap();
    let audit_body: Value = resp.json().await.unwrap();
    let entries = audit_body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "user_created" && e["target_id"] == new_user_id),
        "creating a user must leave an audit_log entry"
    );
}

#[tokio::test]
async fn admin_can_suspend_and_resume_other_users() {
    let admin = TestUser::create_admin().await;
    let target = TestUser::create().await;

    let resp = admin
        .client
        .post(format!("{}/api/v1/users/{}/suspend", base_url(), target.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = admin
        .client
        .post(format!("{}/api/v1/users/{}/resume", base_url(), target.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn general_user_cannot_suspend_others() {
    let general = TestUser::create().await;
    let target = TestUser::create().await;

    let resp = general
        .client
        .post(format!("{}/api/v1/users/{}/suspend", base_url(), target.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn admin_cannot_suspend_their_own_account() {
    let admin = TestUser::create_admin().await;

    let resp = admin
        .client
        .post(format!("{}/api/v1/users/{}/suspend", base_url(), admin.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "an admin must not be able to suspend themselves");
}

#[tokio::test]
async fn general_users_can_still_list_users_for_assignee_dropdowns() {
    let general = TestUser::create().await;

    let resp = general
        .client
        .get(format!("{}/api/v1/users", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "listing users must remain available to non-admins (used by assignee selects)");
    let body: Value = resp.json().await.unwrap();
    assert!(body["items"].as_array().is_some(), "user list must be a paginated envelope");
}

#[tokio::test]
async fn users_list_is_paginated() {
    let admin = TestUser::create_admin().await;

    let resp = admin
        .client
        .get(format!("{}/api/v1/users?page=1&per_page=1", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let page1: Value = resp.json().await.unwrap();
    assert_eq!(page1["page"], 1);
    assert_eq!(page1["per_page"], 1);
    assert_eq!(page1["items"].as_array().unwrap().len(), 1);
    assert!(page1["total"].as_i64().unwrap() >= 2, "at least the admin and a general user should exist by now");
}

#[tokio::test]
async fn admin_can_reset_a_users_password() {
    let admin = TestUser::create_admin().await;
    let target = TestUser::create().await;

    let resp = admin
        .client
        .post(format!("{}/api/v1/users/{}/reset-password", base_url(), target.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let temp_password = body["temporary_password"]
        .as_str()
        .expect("response must include the temporary password")
        .to_string();

    // The target's pre-existing session must be killed by the reset, since
    // whoever controlled it may no longer be the legitimate account owner.
    let resp = target
        .client
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "the target's existing session must be invalidated by the reset");

    let unauth = unauthenticated_client();

    // The user's original password (set by TestUser::create) must no longer work.
    let resp = unauth
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": target.email, "password": "TestPassword123!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "old password must be invalidated by the reset");

    // The freshly issued temporary password must work.
    let resp = unauth
        .post(format!("{}/api/v1/auth/login", base_url()))
        .json(&json!({ "email": target.email, "password": temp_password }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "temporary password must allow login");
}

#[tokio::test]
async fn general_user_cannot_reset_passwords() {
    let general = TestUser::create().await;
    let target = TestUser::create().await;

    let resp = general
        .client
        .post(format!("{}/api/v1/users/{}/reset-password", base_url(), target.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn admin_can_change_a_users_role() {
    let admin = TestUser::create_admin().await;
    let target = TestUser::create().await;

    let resp = admin
        .client
        .patch(format!("{}/api/v1/users/{}/role", base_url(), target.id))
        .json(&json!({ "role": "admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = admin
        .client
        .get(format!("{}/api/v1/users?page=1&per_page=500", base_url()))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    let promoted = items.iter().find(|u| u["id"] == target.id).unwrap();
    assert_eq!(promoted["role"], "admin");

    // And back down again.
    let resp = admin
        .client
        .patch(format!("{}/api/v1/users/{}/role", base_url(), target.id))
        .json(&json!({ "role": "general" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = admin
        .client
        .get(format!("{}/api/v1/users?page=1&per_page=500", base_url()))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    let demoted = items.iter().find(|u| u["id"] == target.id).unwrap();
    assert_eq!(demoted["role"], "general");
}

#[tokio::test]
async fn admin_cannot_change_their_own_role() {
    let admin = TestUser::create_admin().await;

    let resp = admin
        .client
        .patch(format!("{}/api/v1/users/{}/role", base_url(), admin.id))
        .json(&json!({ "role": "general" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn general_user_cannot_change_roles() {
    let general = TestUser::create().await;
    let target = TestUser::create().await;

    let resp = general
        .client
        .patch(format!("{}/api/v1/users/{}/role", base_url(), target.id))
        .json(&json!({ "role": "admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn search_users_matches_display_name_or_email() {
    let general = TestUser::create().await;
    let target_email = format!("{}@example.com", unique("findme"));
    let admin = admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/users", base_url()))
        .json(&json!({
            "email": target_email,
            "password": "TestPassword123!",
            "display_name": "Searchable Person",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let target: Value = resp.json().await.unwrap();
    let target_id = target["id"].as_i64().unwrap();

    // Search by (part of) the display name.
    let resp = general
        .client
        .get(format!("{}/api/v1/users/search?q=Searchable", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let results: Value = resp.json().await.unwrap();
    assert!(results.as_array().unwrap().iter().any(|u| u["id"] == target_id));

    // Search by (part of) the email.
    let resp = general
        .client
        .get(format!("{}/api/v1/users/search?q={}", base_url(), &target_email[..8]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let results: Value = resp.json().await.unwrap();
    assert!(results.as_array().unwrap().iter().any(|u| u["id"] == target_id));

    // A query matching neither should return no results.
    let resp = general
        .client
        .get(format!("{}/api/v1/users/search?q={}", base_url(), unique("nomatch")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let results: Value = resp.json().await.unwrap();
    assert!(results.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn unauthenticated_request_cannot_search_users() {
    let client = unauthenticated_client();

    let resp = client
        .get(format!("{}/api/v1/users/search?q=test", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}
