mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn gantt_endpoint_returns_all_project_tickets_and_dependencies() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("gantt-project")).await;
    let project_id = as_i64(&project, "id");

    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, Some(a_id), "B").await;
    let b_id = as_i64(&b, "id");
    user.create_ticket(project_id, None, "C").await;

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}/gantt", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let data: serde_json::Value = resp.json().await.unwrap();

    let tickets = data["tickets"].as_array().unwrap();
    assert_eq!(tickets.len(), 3, "gantt data must include every ticket in the project");

    let dependencies = data["dependencies"].as_array().unwrap();
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0]["predecessor_ticket_id"], a_id);
    assert_eq!(dependencies[0]["successor_ticket_id"], b_id);
}

#[tokio::test]
async fn gantt_endpoint_requires_authentication() {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/projects/1/gantt", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn gantt_endpoint_returns_empty_for_project_with_no_tickets() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("gantt-empty-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}/gantt", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let data: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(data["tickets"].as_array().unwrap().len(), 0);
    assert_eq!(data["dependencies"].as_array().unwrap().len(), 0);
}
