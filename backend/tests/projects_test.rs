mod common;
use common::{admin_client, as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn create_list_get_update_and_archive_project() {
    let user = TestUser::create().await;
    let name = unique("project");

    let created = user.create_project(&name).await;
    let project_id = as_i64(&created, "id");
    assert_eq!(created["name"], name);
    assert!(created["archived_at"].is_null());

    let resp = user
        .client
        .get(format!("{}/api/v1/projects", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id));

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .patch(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .json(&json!({ "description": "updated description" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(updated["description"], "updated description");
    assert_eq!(updated["name"], name, "COALESCE update must not clobber name");

    // Archiving is admin-only, not delegated to the project's own manager.
    let admin = admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/projects/{}/archive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = user
        .client
        .get(format!("{}/api/v1/projects", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(
        !list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id),
        "archived projects should be excluded from the default list"
    );
}

#[tokio::test]
async fn only_admins_can_archive_or_unarchive_a_project() {
    let user = TestUser::create().await;
    let name = unique("project-archive-perms");
    let created = user.create_project(&name).await;
    let project_id = as_i64(&created, "id");

    // The project's own manager (a general, non-admin user) must not be
    // able to archive it - only a site admin can.
    let resp = user
        .client
        .post(format!("{}/api/v1/projects/{}/archive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let admin = admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/projects/{}/archive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = user
        .client
        .post(format!("{}/api/v1/projects/{}/unarchive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = admin
        .post(format!("{}/api/v1/projects/{}/unarchive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn archived_project_is_read_only_and_reappears_after_unarchiving() {
    let user = TestUser::create().await;
    let name = unique("project-readonly");
    let created = user.create_project(&name).await;
    let project_id = as_i64(&created, "id");

    let admin = admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/projects/{}/archive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Reads still work while archived.
    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Writes are rejected, even for the project's own manager...
    let resp = user
        .client
        .patch(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .json(&json!({ "description": "should not apply" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "an archived project must reject writes to its own settings");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({ "project_id": project_id, "title": unique("archived-project-ticket") }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "an archived project must reject creating new tickets");

    // ...and even for a site admin, until they explicitly unarchive it.
    let resp = admin
        .patch(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .json(&json!({ "description": "still should not apply" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "archived read-only applies to admins too, not just non-admin members");

    // Appears in the "archived" tab, not the default list.
    let resp = user
        .client
        .get(format!("{}/api/v1/projects?archived=true&per_page=500", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id));

    let resp = admin
        .post(format!("{}/api/v1/projects/{}/unarchive", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Writes work again, and it's back in the default list instead of the
    // archived tab.
    let resp = user
        .client
        .patch(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .json(&json!({ "description": "applies now" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/projects?per_page=500", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id));

    let resp = user
        .client
        .get(format!("{}/api/v1/projects?archived=true&per_page=500", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(!list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id));
}

#[tokio::test]
async fn project_list_is_paginated() {
    let user = TestUser::create().await;
    let prefix = unique("paginated-project");
    let name_a = format!("{prefix}-a");
    let name_b = format!("{prefix}-b");
    let name_c = format!("{prefix}-c");

    user.create_project(&name_a).await;
    user.create_project(&name_b).await;
    user.create_project(&name_c).await;

    let resp = user
        .client
        .get(format!("{}/api/v1/projects?page=1&per_page=2", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let page1: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(page1["page"], 1);
    assert_eq!(page1["per_page"], 2);
    assert_eq!(page1["items"].as_array().unwrap().len(), 2);
    let total_before = page1["total"].as_i64().unwrap();
    assert!(total_before >= 3);

    let resp = user
        .client
        .get(format!("{}/api/v1/projects?page=2&per_page=2", base_url()))
        .send()
        .await
        .unwrap();
    let page2: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(page2["page"], 2);
    // The two pages must not overlap.
    let page1_ids: Vec<i64> = page1["items"].as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).collect();
    let page2_ids: Vec<i64> = page2["items"].as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).collect();
    assert!(page1_ids.iter().all(|id| !page2_ids.contains(id)));
}

#[tokio::test]
async fn duplicate_project_name_is_rejected() {
    let user = TestUser::create().await;
    let name = unique("dup-project");

    user.create_project(&name).await;

    // Project creation is admin-only, so the duplicate-name attempt itself
    // must go through an admin session too.
    let admin = admin_client().await;
    let resp = admin
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({ "name": name, "description": null }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn only_admins_can_create_projects() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({ "name": unique("general-user-project"), "description": null }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403, "a general user must not be able to create a project directly");
}

#[tokio::test]
async fn creating_a_project_with_initial_members_adds_them_instead_of_the_creator() {
    let manager = TestUser::create().await;
    let viewer = TestUser::create().await;
    let admin = admin_client().await;

    let resp = admin
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({
            "name": unique("project-with-members"),
            "description": null,
            "members": [
                { "user_id": manager.id, "role": "manager" },
                { "user_id": viewer.id, "role": "viewer" },
            ],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let project: serde_json::Value = resp.json().await.unwrap();
    let project_id = as_i64(&project, "id");

    let resp = manager
        .client
        .get(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let members: serde_json::Value = resp.json().await.unwrap();
    let members = members.as_array().unwrap();

    assert_eq!(members.len(), 2, "only the explicitly listed members should be added, not the admin creator");
    assert!(members.iter().any(|m| m["user"]["id"] == manager.id && m["role"] == "manager"));
    assert!(members.iter().any(|m| m["user"]["id"] == viewer.id && m["role"] == "viewer"));
}

#[tokio::test]
async fn creating_a_project_with_members_but_no_manager_is_rejected() {
    let viewer = TestUser::create().await;
    let admin = admin_client().await;

    let resp = admin
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({
            "name": unique("project-no-manager"),
            "description": null,
            "members": [{ "user_id": viewer.id, "role": "viewer" }],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400, "a project must always be created with at least one manager");
}

#[tokio::test]
async fn admin_default_project_list_only_shows_projects_they_are_involved_in() {
    let manager = TestUser::create().await;
    let admin = admin_client().await;

    let name = unique("admin-not-involved");
    let resp = admin
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({
            "name": name,
            "description": null,
            "members": [{ "user_id": manager.id, "role": "manager" }],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let project_id = as_i64(&resp.json().await.unwrap(), "id");

    // Default (no scope, i.e. "mine") admin listing should not include a
    // project the admin has no membership in.
    let resp = admin
        .get(format!("{}/api/v1/projects?per_page=500", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(
        !list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id),
        "an admin's default project list should only show projects they're involved in"
    );

    // scope=all restores full visibility for admins.
    let resp = admin
        .get(format!("{}/api/v1/projects?per_page=500&scope=all", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(
        list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id),
        "scope=all should let an admin see every project regardless of membership"
    );

    // scope=all is a no-op for a general user - a project they have no
    // membership in (the one above, since `manager` isn't a member of it)
    // must stay invisible even when they pass scope=all.
    let outsider = TestUser::create().await;
    let resp = outsider
        .client
        .get(format!("{}/api/v1/projects?per_page=500&scope=all", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    assert!(
        !list["items"].as_array().unwrap().iter().any(|p| p["id"] == project_id),
        "scope=all must not grant a general user visibility into projects they aren't a member of"
    );
}

#[tokio::test]
async fn creating_and_updating_a_project_is_recorded_in_the_audit_log() {
    let user = TestUser::create().await;
    let admin = admin_client().await;
    let admin_id = admin
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()["user"]["id"]
        .as_i64()
        .unwrap();

    let resp = admin
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({
            "name": unique("audited-project"),
            "description": null,
            "members": [{ "user_id": user.id, "role": "manager" }],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let project_id = as_i64(&resp.json().await.unwrap(), "id");

    let resp = user
        .client
        .patch(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .json(&json!({ "description": "audited update" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), admin_id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "project_created" && e["target_id"] == project_id),
        "creating a project must leave an audit_log entry"
    );

    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "project_updated" && e["target_id"] == project_id),
        "updating a project must leave an audit_log entry"
    );
}

#[tokio::test]
async fn get_nonexistent_project_returns_404() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/999999999", base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}
