mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::{json, Value};

async fn notifications_for(user: &TestUser) -> Vec<Value> {
    let resp = user
        .client
        .get(format!("{}/api/v1/notifications", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    body["items"].as_array().unwrap().clone()
}

fn find_notification<'a>(items: &'a [Value], notif_type: &str) -> Option<&'a Value> {
    items.iter().find(|n| n["notif_type"] == notif_type)
}

async fn move_status(user: &TestUser, ticket_id: i64, status_id: i64) -> reqwest::StatusCode {
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": status_id }))
        .send()
        .await
        .unwrap();
    resp.status()
}

#[tokio::test]
async fn moving_a_ticket_to_done_notifies_the_projects_manager() {
    let manager = TestUser::create().await;
    let member = TestUser::create().await;
    let project = manager.create_project(&unique("confirm-notify-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = manager
        .client
        .post(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .json(&json!({ "user_id": member.id, "role": "member" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let ticket = member.create_ticket(project_id, None, "Ticket to complete").await;
    let ticket_id = as_i64(&ticket, "id");

    let in_progress_id = member.status_id(project_id, "in_progress").await;
    let done_id = member.status_id(project_id, "done").await;

    assert_eq!(move_status(&member, ticket_id, in_progress_id).await, 200);
    assert_eq!(move_status(&member, ticket_id, done_id).await, 200);

    let items = notifications_for(&manager).await;
    let notif = find_notification(&items, "ticket_confirmation_requested")
        .expect("the manager should be notified that a ticket needs confirmation");
    assert_eq!(notif["ticket_id"], ticket_id);
}

#[tokio::test]
async fn only_a_project_manager_can_close_a_done_ticket() {
    let manager = TestUser::create().await;
    let member = TestUser::create().await;
    let project = manager.create_project(&unique("confirm-permission-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = manager
        .client
        .post(format!("{}/api/v1/projects/{}/members", base_url(), project_id))
        .json(&json!({ "user_id": member.id, "role": "member" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let ticket = member.create_ticket(project_id, None, "Ticket to close").await;
    let ticket_id = as_i64(&ticket, "id");

    let in_progress_id = member.status_id(project_id, "in_progress").await;
    let done_id = member.status_id(project_id, "done").await;
    let closed_id = member.status_id(project_id, "closed").await;

    assert_eq!(move_status(&member, ticket_id, in_progress_id).await, 200);
    assert_eq!(move_status(&member, ticket_id, done_id).await, 200);

    assert_eq!(
        move_status(&member, ticket_id, closed_id).await,
        403,
        "a plain member must not be able to close (confirm) a done ticket"
    );

    assert_eq!(
        move_status(&manager, ticket_id, closed_id).await,
        200,
        "the project manager must be able to close (confirm) a done ticket"
    );
}

#[tokio::test]
async fn a_manager_closing_their_own_completed_ticket_is_not_self_notified() {
    let manager = TestUser::create().await;
    let project = manager.create_project(&unique("confirm-self-project")).await;
    let project_id = as_i64(&project, "id");

    let ticket = manager.create_ticket(project_id, None, "Solo ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let in_progress_id = manager.status_id(project_id, "in_progress").await;
    let done_id = manager.status_id(project_id, "done").await;

    assert_eq!(move_status(&manager, ticket_id, in_progress_id).await, 200);
    assert_eq!(move_status(&manager, ticket_id, done_id).await, 200);

    let items = notifications_for(&manager).await;
    assert!(
        find_notification(&items, "ticket_confirmation_requested").is_none(),
        "a manager completing their own ticket must not notify themselves"
    );
}
