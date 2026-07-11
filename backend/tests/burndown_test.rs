mod common;
use chrono::{Duration, Utc};
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn burndown_reflects_actual_remaining_ticket_count_after_status_changes() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("burndown-project")).await;
    let project_id = as_i64(&project, "id");

    let today = Utc::now().date_naive();
    let due_date = today + Duration::days(7);

    let resp = user
        .client
        .post(format!("{}/api/v1/projects/{}/milestones", base_url(), project_id))
        .json(&json!({
            "name": "burndown milestone",
            "start_date": today.to_string(),
            "due_date": due_date.to_string(),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let milestone: serde_json::Value = resp.json().await.unwrap();
    let milestone_id = as_i64(&milestone, "id");
    assert_eq!(milestone["start_date"], today.to_string());

    // Three tickets in the milestone: one fully done, one in progress, one
    // untouched - so "remaining" should end up at 2.
    let done_ticket = user.create_ticket(project_id, None, "will be done").await;
    let in_progress_ticket = user.create_ticket(project_id, None, "will be in progress").await;
    let untouched_ticket = user.create_ticket(project_id, None, "left alone").await;

    for ticket in [&done_ticket, &in_progress_ticket, &untouched_ticket] {
        let ticket_id = as_i64(ticket, "id");
        let resp = user
            .client
            .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
            .json(&json!({ "milestone_id": milestone_id }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    let in_progress_status_id = user.status_id(project_id, "in_progress").await;
    let done_status_id = user.status_id(project_id, "done").await;

    let done_ticket_id = as_i64(&done_ticket, "id");
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), done_ticket_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), done_ticket_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let in_progress_ticket_id = as_i64(&in_progress_ticket, "id");
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), in_progress_ticket_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/milestones/{}/burndown",
            base_url(),
            project_id,
            milestone_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let data: serde_json::Value = resp.json().await.unwrap();

    assert_eq!(data["start_date"], today.to_string());
    assert_eq!(data["total_at_start"], 3, "all 3 tickets already existed at the milestone's start date");

    let points = data["points"].as_array().unwrap();
    assert!(!points.is_empty(), "expected at least today's point to be present");
    let last_point = points.last().unwrap();
    assert_eq!(last_point["date"], today.to_string());
    assert_eq!(
        last_point["remaining"], 2,
        "only the untouched and in-progress tickets should still count as remaining"
    );
}

#[tokio::test]
async fn burndown_returns_empty_points_when_milestone_has_no_start_date() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("burndown-no-start-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/projects/{}/milestones", base_url(), project_id))
        .json(&json!({ "name": "no start date milestone" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let milestone: serde_json::Value = resp.json().await.unwrap();
    let milestone_id = as_i64(&milestone, "id");
    assert!(milestone["start_date"].is_null());

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/milestones/{}/burndown",
            base_url(),
            project_id,
            milestone_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let data: serde_json::Value = resp.json().await.unwrap();
    assert!(data["start_date"].is_null());
    assert_eq!(data["points"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn burndown_for_nonexistent_milestone_is_404() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("burndown-404-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/milestones/999999999/burndown",
            base_url(),
            project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
