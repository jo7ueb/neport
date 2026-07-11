mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn create_and_list_comments_with_author_from_session() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("comment-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket with comments").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": "first comment" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let comment: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(comment["author_id"], user.id, "author_id must come from the session, never the request body");
    assert_eq!(comment["body"], "first comment");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": "second comment" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 2);
    let comments = body["items"].as_array().unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0]["body"], "first comment", "comments must be ordered oldest first");
    assert_eq!(comments[1]["body"], "second comment");
}

#[tokio::test]
async fn comments_list_is_paginated() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("comment-page-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket with many comments").await;
    let ticket_id = as_i64(&ticket, "id");

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
        .get(format!("{}/api/v1/tickets/{}/comments?page=1&per_page=2", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/comments?page=2&per_page=2", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["items"].as_array().unwrap().len(), 1, "the third comment should be on the second page");
}
