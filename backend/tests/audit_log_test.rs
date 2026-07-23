mod common;
use common::{admin_client, as_i64, base_url, unique, TestUser};
use serde_json::{json, Value};

async fn audit_entries_for(actor_id: i64) -> Vec<Value> {
    let admin = admin_client().await;
    let resp = admin
        .get(format!("{}/api/v1/audit-log", base_url()))
        .query(&[("user_id", actor_id.to_string()), ("per_page", "200".to_string())])
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    body["items"].as_array().unwrap().clone()
}

fn find<'a>(entries: &'a [Value], action: &str) -> Option<&'a Value> {
    entries.iter().find(|e| e["action"] == action)
}

#[tokio::test]
async fn commenting_on_a_ticket_appears_in_the_unified_audit_log() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("audit-comment-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": "hello from the audit log test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let entries = audit_entries_for(user.id).await;
    let entry = find(&entries, "ticket_commented").expect("comment should appear in the unified audit log");
    assert_eq!(entry["target_id"], ticket_id);
    assert!(entry["to_text"].as_str().unwrap().contains("hello from the audit log test"));
}

#[tokio::test]
async fn ticket_field_and_status_changes_appear_in_the_unified_audit_log() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("audit-ticket-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Original title").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "title": "Updated title" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let entries = audit_entries_for(user.id).await;
    find(&entries, "ticket_created").expect("ticket creation should appear in the unified audit log");
    let changed = find(&entries, "ticket_title_changed").expect("title change should appear in the unified audit log");
    assert_eq!(changed["from_text"], "Original title");
    assert_eq!(changed["to_text"], "Updated title");
}

#[tokio::test]
async fn adding_and_removing_a_ticket_dependency_appears_in_the_unified_audit_log() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("audit-dependency-project")).await;
    let project_id = as_i64(&project, "id");
    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, None, "B").await;
    let b_id = as_i64(&b, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let dependency: Value = resp.json().await.unwrap();
    let dependency_id = as_i64(&dependency, "id");

    let entries = audit_entries_for(user.id).await;
    find(&entries, "dependency_added").expect("adding a dependency should appear in the unified audit log");

    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}/dependencies/{}", base_url(), a_id, dependency_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let entries = audit_entries_for(user.id).await;
    find(&entries, "dependency_removed").expect("removing a dependency should appear in the unified audit log");
}

#[tokio::test]
async fn starting_and_ending_a_work_session_appears_in_the_unified_audit_log() {
    let user = TestUser::create().await;

    let resp = user.client.post(format!("{}/api/v1/work-sessions/start", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let resp = user.client.post(format!("{}/api/v1/work-sessions/end", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let entries = audit_entries_for(user.id).await;
    find(&entries, "work_session_started").expect("starting a work session should appear in the unified audit log");
    find(&entries, "work_session_ended").expect("ending a work session should appear in the unified audit log");
}

#[tokio::test]
async fn submitting_and_approving_a_daily_report_appears_in_the_unified_audit_log() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;

    user_ends_a_valid_day(&subject).await;

    let date = common::now_jst().format("%Y-%m-%d").to_string();
    let resp = approver
        .client
        .post(format!("{}/api/v1/approvals/subordinates/{}/day/{}/decide", base_url(), subject.id, date))
        .json(&json!({ "decision": "approved", "comment": "looks good" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let entries = audit_entries_for(approver.id).await;
    let entry = find(&entries, "report_approved").expect("report approval should appear in the unified audit log");
    assert_eq!(entry["target_id"], subject.id);
}

async fn user_ends_a_valid_day(user: &TestUser) {
    user.client.post(format!("{}/api/v1/work-sessions/start", base_url())).send().await.unwrap();
    let resp = user.client.post(format!("{}/api/v1/work-sessions/end", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "did some work" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn uploading_an_attachment_appears_in_the_unified_audit_log() {
    let user = TestUser::create().await;

    // Attachments are content-addressed by sha256 with `ON CONFLICT (sha256)
    // DO NOTHING` (see backend/src/attachments/repo.rs) - reusing fixed bytes
    // across test runs would silently no-op against a row some earlier run
    // already inserted (keeping its original uploaded_by_id), so the content
    // must be unique per run.
    let content = unique("attachment-content").into_bytes();
    let part = reqwest::multipart::Part::bytes(content).file_name("test.png").mime_str("image/png").unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = user
        .client
        .post(format!("{}/api/v1/attachments", base_url()))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let entries = audit_entries_for(user.id).await;
    find(&entries, "attachment_uploaded").expect("uploading an attachment should appear in the unified audit log");
}

#[tokio::test]
async fn assigning_an_approver_appears_in_the_unified_audit_log() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;

    let admin = admin_client().await;
    let entries_resp = admin
        .get(format!("{}/api/v1/audit-log", base_url()))
        .query(&[("per_page", "200")])
        .send()
        .await
        .unwrap();
    assert_eq!(entries_resp.status(), 200);
    let body: Value = entries_resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    let entry = entries
        .iter()
        .find(|e| e["action"] == "approver_assigned" && e["target_id"] == subject.id)
        .expect("assigning an approver should appear in the unified audit log");
    assert_eq!(entry["to_text"], "Test User");
}

#[tokio::test]
async fn filtering_by_actor_does_not_mix_in_another_users_entries() {
    let a = TestUser::create().await;
    let b = TestUser::create().await;
    let project = a.create_project(&unique("audit-filter-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = a.create_ticket(project_id, None, "A's ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = a
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": "from A" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let b_entries = audit_entries_for(b.id).await;
    assert!(
        !b_entries.iter().any(|e| e["action"] == "ticket_commented" && e["target_id"] == ticket_id),
        "filtering by user B must not show user A's comment"
    );
}
