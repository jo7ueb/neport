mod common;
use common::{admin_client, as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn create_list_update_delete_milestone_and_link_it_to_a_ticket() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("milestone-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .post(format!(
            "{}/api/v1/projects/{}/milestones",
            base_url(),
            project_id
        ))
        .json(&json!({ "name": "v1.0", "due_date": "2026-12-31", "description": "first release" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let milestone: serde_json::Value = resp.json().await.unwrap();
    let milestone_id = as_i64(&milestone, "id");
    assert_eq!(milestone["project_id"], project_id);

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/milestones",
            base_url(),
            project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    let resp = user
        .client
        .patch(format!(
            "{}/api/v1/projects/{}/milestones/{}",
            base_url(),
            project_id,
            milestone_id
        ))
        .json(&json!({ "name": "v1.0-final" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(updated["name"], "v1.0-final");

    let admin = admin_client().await;
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "milestone_created" && e["target_id"] == milestone_id),
        "creating a milestone must leave an audit_log entry"
    );
    assert!(
        entries.iter().any(|e| e["action"] == "milestone_updated" && e["target_id"] == milestone_id),
        "updating a milestone must leave an audit_log entry"
    );

    let ticket = user.create_ticket(project_id, None, "Ticket for milestone").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "milestone_id": milestone_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let linked: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(linked["milestone_id"], milestone_id);

    let resp = user
        .client
        .delete(format!(
            "{}/api/v1/projects/{}/milestones/{}",
            base_url(),
            project_id,
            milestone_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "milestone_deleted" && e["target_id"] == milestone_id),
        "deleting a milestone must leave an audit_log entry"
    );

    // Milestone deletion is a soft delete (see milestones::repo::delete_milestone)
    // specifically so a ticket that referenced it keeps that historical link
    // instead of silently losing it - the milestone just stops showing up in
    // the project's active milestone list.
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    let ticket_after_delete: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        ticket_after_delete["milestone_id"], milestone_id,
        "soft-deleting a milestone must not clear tickets' historical milestone_id"
    );

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/milestones",
            base_url(),
            project_id
        ))
        .send()
        .await
        .unwrap();
    let list_after_delete: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        list_after_delete.as_array().unwrap().len(),
        0,
        "a soft-deleted milestone must not appear in the project's active list"
    );
}

#[tokio::test]
async fn milestone_start_date_round_trips_through_create_and_update() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("milestone-start-date-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/projects/{}/milestones", base_url(), project_id))
        .json(&json!({ "name": "v2.0", "start_date": "2026-01-01", "due_date": "2026-03-31" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let milestone: serde_json::Value = resp.json().await.unwrap();
    let milestone_id = as_i64(&milestone, "id");
    assert_eq!(milestone["start_date"], "2026-01-01");

    let resp = user
        .client
        .patch(format!(
            "{}/api/v1/projects/{}/milestones/{}",
            base_url(),
            project_id,
            milestone_id
        ))
        .json(&json!({ "start_date": "2026-01-15" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(updated["start_date"], "2026-01-15");
}
