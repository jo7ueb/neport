mod common;
use common::{as_i64, base_url, now_jst, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn ticket_activity_log_records_created_comment_and_status_change() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("activity-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket with activity").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": "a comment" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // The standard workflow is a ladder now, so "open" -> "done" must go
    // through "in_progress" first.
    let in_progress_status_id = user.status_id(project_id, "in_progress").await;
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let done_status_id = user.status_id(project_id, "done").await;
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/activity", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let types: Vec<String> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["activity_type"].as_str().unwrap().to_string())
        .collect();

    assert!(types.contains(&"created".to_string()));
    assert!(types.contains(&"comment".to_string()));
    assert!(types.contains(&"status_change".to_string()));
    assert_eq!(body["total"], types.len() as i64);
}

#[tokio::test]
async fn ticket_activity_log_is_paginated() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("activity-page-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket with lots of activity").await;
    let ticket_id = as_i64(&ticket, "id");

    // "created" is already one activity entry; add a few more.
    for i in 0..3 {
        let resp = user
            .client
            .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
            .json(&json!({ "body": format!("comment {i}") }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201);
    }

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/activity?page=1&per_page=2", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4, "1 created + 3 comments");
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn user_activity_endpoint_filters_by_date() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("activity-user-project")).await;
    let project_id = as_i64(&project, "id");
    user.create_ticket(project_id, None, "Ticket today").await;

    let today = now_jst().format("%Y-%m-%d").to_string();
    let resp = user
        .client
        .get(format!(
            "{}/api/v1/users/{}/activity?date={}",
            base_url(),
            user.id,
            today
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let activity: serde_json::Value = resp.json().await.unwrap();
    assert!(!activity.as_array().unwrap().is_empty());

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/users/{}/activity?date=2000-01-01",
            base_url(),
            user.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let activity: serde_json::Value = resp.json().await.unwrap();
    assert!(
        activity.as_array().unwrap().is_empty(),
        "a far past date should return no activity"
    );
}
