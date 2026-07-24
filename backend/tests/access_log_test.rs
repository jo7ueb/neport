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

// Every write path this test touches is a domain table that used to be
// merged into the unified audit log with a hardcoded NULL ip_address (see
// audit::repo::ENTRIES_SQL) - they now go through the shared access_log
// table instead, so this asserts the real IP makes it all the way through.

#[tokio::test]
async fn commenting_on_a_ticket_records_the_clients_ip() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("access-log-comment-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .header("X-Forwarded-For", "203.0.113.10")
        .json(&json!({ "body": "hello with an ip" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let entries = audit_entries_for(user.id).await;
    let entry = find(&entries, "ticket_commented").expect("comment should appear in the unified audit log");
    assert_eq!(entry["ip_address"], "203.0.113.10");
}

#[tokio::test]
async fn adding_and_removing_a_dependency_records_separate_ips_per_event() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("access-log-dependency-project")).await;
    let project_id = as_i64(&project, "id");
    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, None, "B").await;
    let b_id = as_i64(&b, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .header("X-Forwarded-For", "203.0.113.20")
        .json(&json!({ "direction": "successor", "other_ticket_id": b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let dependency: Value = resp.json().await.unwrap();
    let dependency_id = as_i64(&dependency, "id");

    let entries = audit_entries_for(user.id).await;
    let added = find(&entries, "dependency_added").expect("adding a dependency should appear in the unified audit log");
    assert_eq!(added["ip_address"], "203.0.113.20");

    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}/dependencies/{}", base_url(), a_id, dependency_id))
        .header("X-Forwarded-For", "203.0.113.21")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let entries = audit_entries_for(user.id).await;
    let removed = find(&entries, "dependency_removed").expect("removing a dependency should appear in the unified audit log");
    assert_eq!(removed["ip_address"], "203.0.113.21");
    // The add event's own IP must still be the original one, not overwritten
    // by the later delete - proves created_access_log_id/deleted_access_log_id
    // are genuinely independent columns.
    let added = find(&entries, "dependency_added").unwrap();
    assert_eq!(added["ip_address"], "203.0.113.20");
}

#[tokio::test]
async fn starting_and_ending_a_work_session_records_separate_ips_per_event() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .header("X-Forwarded-For", "203.0.113.30")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .header("X-Forwarded-For", "203.0.113.31")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let entries = audit_entries_for(user.id).await;
    let started = find(&entries, "work_session_started").expect("start should appear in the unified audit log");
    assert_eq!(started["ip_address"], "203.0.113.30");
    let ended = find(&entries, "work_session_ended").expect("end should appear in the unified audit log");
    assert_eq!(ended["ip_address"], "203.0.113.31");
}

#[tokio::test]
async fn approving_a_report_records_the_approvers_ip() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;

    subject.client.post(format!("{}/api/v1/work-sessions/start", base_url())).send().await.unwrap();
    let resp = subject.client.post(format!("{}/api/v1/work-sessions/end", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let resp = subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "did some work" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let date = common::now_jst().format("%Y-%m-%d").to_string();
    let resp = approver
        .client
        .post(format!("{}/api/v1/approvals/subordinates/{}/day/{}/decide", base_url(), subject.id, date))
        .header("X-Forwarded-For", "203.0.113.40")
        .json(&json!({ "decision": "approved", "comment": "looks good" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let entries = audit_entries_for(approver.id).await;
    let entry = find(&entries, "report_approved").expect("report approval should appear in the unified audit log");
    assert_eq!(entry["ip_address"], "203.0.113.40");
}

#[tokio::test]
async fn uploading_an_attachment_records_the_uploaders_ip() {
    let user = TestUser::create().await;

    let content = unique("access-log-attachment-content").into_bytes();
    let part = reqwest::multipart::Part::bytes(content).file_name("test.png").mime_str("image/png").unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = user
        .client
        .post(format!("{}/api/v1/attachments", base_url()))
        .header("X-Forwarded-For", "203.0.113.50")
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let entries = audit_entries_for(user.id).await;
    let entry = find(&entries, "attachment_uploaded").expect("upload should appear in the unified audit log");
    assert_eq!(entry["ip_address"], "203.0.113.50");
}

// Regression: audit_log-sourced actions (this app's original, non-merged
// audit trail) must keep reporting a real IP now that they go through
// access_log instead of writing straight to audit_log.ip_address.
#[tokio::test]
async fn an_audit_log_sourced_action_still_records_the_actors_ip() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("access-log-ticket-delete-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = user.create_ticket(project_id, None, "Doomed ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .header("X-Forwarded-For", "203.0.113.60")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let entries = audit_entries_for(user.id).await;
    let entry = find(&entries, "ticket_deleted").expect("ticket deletion should appear in the unified audit log");
    assert_eq!(entry["ip_address"], "203.0.113.60");
}
