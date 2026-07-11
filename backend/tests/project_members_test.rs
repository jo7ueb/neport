mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn creator_becomes_manager_and_non_members_are_forbidden() {
    let owner = TestUser::create().await;
    let outsider = TestUser::create().await;

    let project = owner.create_project(&unique("members-project")).await;
    let project_id = as_i64(&project, "id");

    // The creator can read their own project...
    let resp = owner
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // ...but a completely unrelated user cannot, since this project now has
    // a member (the creator) and is therefore restricted.
    let resp = outsider
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "non-members must not be able to read a restricted project");

    let resp = outsider
        .client
        .get(format!("{}/api/v1/projects/{}/tickets", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "non-members must not be able to list tickets either");
}

#[tokio::test]
async fn viewer_can_read_but_not_write_member_can_write_but_not_manage() {
    let owner = TestUser::create().await;
    let viewer = TestUser::create().await;
    let member = TestUser::create().await;

    let project = owner.create_project(&unique("roles-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = owner
        .client
        .post(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .json(&json!({ "user_id": viewer.id, "role": "viewer" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = owner
        .client
        .post(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .json(&json!({ "user_id": member.id, "role": "member" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Viewer: read ok, write forbidden.
    let resp = viewer
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = viewer
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({ "project_id": project_id, "title": "should be rejected" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "a viewer must not be able to create tickets");

    // Member: read+write ok, but cannot manage membership.
    let ticket = member.create_ticket(project_id, None, "member can create this").await;
    let ticket_id = as_i64(&ticket, "id");
    assert_eq!(ticket["reporter_id"], member.id);

    let resp = member
        .client
        .post(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .json(&json!({ "user_id": viewer.id, "role": "manager" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "a member must not be able to manage project membership");

    let resp = member
        .client
        .patch(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .json(&json!({ "name": unique("renamed-by-member") }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "a member must not be able to edit project settings");

    // Manager (the owner) can still do everything, including deleting the
    // ticket the member created.
    let resp = owner
        .client
        .delete(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn removing_the_last_manager_is_rejected() {
    let owner = TestUser::create().await;
    let project = owner.create_project(&unique("last-manager-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = owner
        .client
        .delete(format!("{}/api/v1/projects/{}/members/{}", base_url(), project_id, owner.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "the project's only manager must not be removable");

    let resp = owner
        .client
        .patch(format!("{}/api/v1/projects/{}/members/{}", base_url(), project_id, owner.id))
        .json(&json!({ "role": "member" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "demoting the only manager must also be rejected");
}

#[tokio::test]
async fn site_admin_bypasses_project_membership_entirely() {
    let owner = TestUser::create().await;
    let admin = TestUser::create_admin().await;

    let project = owner.create_project(&unique("admin-bypass-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = admin
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "a site admin can read any project regardless of membership");

    let resp = admin
        .client
        .post(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .json(&json!({ "user_id": admin.id, "role": "viewer" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "a site admin can manage membership even without being a member");
}
