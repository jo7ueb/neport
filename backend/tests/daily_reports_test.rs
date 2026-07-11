mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn today_tickets_auto_collects_touched_tickets() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("daily-report-project")).await;
    let project_id = as_i64(&project, "id");

    let ticket = user.create_ticket(project_id, None, "Touched ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": "worked on this" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = user
        .client
        .get(format!("{}/api/v1/daily-reports/today/tickets", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let tickets: serde_json::Value = resp.json().await.unwrap();
    let ids: Vec<i64> = tickets.as_array().unwrap().iter().map(|t| t["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&ticket_id));
}

#[tokio::test]
async fn submit_report_upserts_and_appears_in_history() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({
            "achievement": "shipped feature X",
            "leftover": "review pending",
            "keep": "good pairing session",
            "problem": "flaky test",
            "try": "add retries"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let report: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(report["achievement"], "shipped feature X");
    assert_eq!(report["try"], "add retries");

    // Re-submitting the same day must update in place (upsert), not duplicate.
    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "shipped feature X, updated" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/daily-reports", base_url()))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let history = body["items"].as_array().unwrap();
    assert_eq!(history.len(), 1, "upsert must not create a duplicate row for the same date");
    assert_eq!(history[0]["achievement"], "shipped feature X, updated");
}

#[tokio::test]
async fn daily_report_history_is_paginated() {
    let user = TestUser::create().await;

    for i in 0..3 {
        let date = format!("2020-01-0{}", i + 1);
        let resp = user
            .client
            .post(format!("{}/api/v1/daily-reports", base_url()))
            .json(&json!({ "report_date": date, "achievement": format!("day {i}") }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    let resp = user
        .client
        .get(format!("{}/api/v1/daily-reports?page=1&per_page=2", base_url()))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn submitting_report_after_ending_work_makes_the_day_valid_on_next_read() {
    let user = TestUser::create().await;

    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    let today: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(today["status"], "invalid", "ending work without a report must be invalid");

    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "submitted after ending work" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    let today: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(today["status"], "valid", "status is recomputed fresh, so a late report is reflected immediately");
}

#[tokio::test]
async fn ending_work_after_report_already_submitted_is_valid_immediately() {
    let user = TestUser::create().await;

    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();

    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "submitted before ending work" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    let today: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(today["status"], "valid");
}

#[tokio::test]
async fn get_report_for_date_without_submission_returns_404() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .get(format!("{}/api/v1/daily-reports/2000-01-01", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
