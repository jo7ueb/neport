mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::{json, Value};
use std::time::Duration;

async fn actual_duration_seconds(user: &TestUser, ticket_id: i64) -> i64 {
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/actual-duration", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    body["in_progress_seconds"].as_i64().unwrap()
}

async fn set_status(user: &TestUser, ticket_id: i64, status_id: i64) {
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn a_ticket_that_never_left_its_initial_todo_status_has_zero_actual_duration() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("duration-todo-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    assert_eq!(actual_duration_seconds(&user, ticket_id).await, 0);
}

#[tokio::test]
async fn actual_duration_accumulates_while_in_progress_and_freezes_once_done() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("duration-flow-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let in_progress_id = user.status_id(project_id, "in_progress").await;
    let done_id = user.status_id(project_id, "done").await;

    set_status(&user, ticket_id, in_progress_id).await;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    set_status(&user, ticket_id, done_id).await;

    let frozen = actual_duration_seconds(&user, ticket_id).await;
    assert!(frozen >= 1, "should have accrued at least the ~1.1s spent in_progress, got {frozen}");

    // Once done, the duration must not keep growing with the passage of time.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let still_frozen = actual_duration_seconds(&user, ticket_id).await;
    assert_eq!(still_frozen, frozen, "actual duration must stop accruing once the ticket is done");
}

#[tokio::test]
async fn pausing_back_to_todo_does_not_count_toward_actual_duration() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("duration-pause-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let open_id = user.status_id(project_id, "open").await;
    let in_progress_id = user.status_id(project_id, "in_progress").await;

    set_status(&user, ticket_id, in_progress_id).await;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    set_status(&user, ticket_id, open_id).await; // pause

    let after_pause = actual_duration_seconds(&user, ticket_id).await;
    assert!(after_pause >= 1);

    // Sitting paused in a todo-category status must not accrue more time.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let still_after_pause = actual_duration_seconds(&user, ticket_id).await;
    assert_eq!(still_after_pause, after_pause, "time spent paused (todo-category) must not count");
}
