mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn new_project_defaults_to_the_standard_workflow_and_kanban_reads_its_statuses() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("workflow-default-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}/workflow", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let detail: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(detail["workflow"]["is_default"], true);

    let statuses = detail["statuses"].as_array().unwrap();
    let keys: Vec<&str> = statuses.iter().map(|s| s["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"open"));
    assert!(keys.contains(&"in_progress"));
    assert!(keys.contains(&"done"));
    assert!(keys.contains(&"closed"));

    // Kanban is now purely the workflow's status list - a fresh ticket
    // should land on the initial status ("open").
    let ticket = user.create_ticket(project_id, None, "Kanban card").await;
    assert_eq!(ticket["status"]["key"], "open");

    // "Moving" a card is just updating status_id via the regular ticket
    // update endpoint - open -> in_progress is a valid ladder step.
    let in_progress_id = user.status_id(project_id, "in_progress").await;
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), as_i64(&ticket, "id")))
        .json(&json!({ "status_id": in_progress_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let moved: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(moved["status"]["key"], "in_progress");
}

#[tokio::test]
async fn custom_workflow_enforces_its_declared_transitions_only() {
    let admin = TestUser::create_admin().await;

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows", base_url()))
        .json(&json!({ "name": unique("custom-workflow") }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let workflow: serde_json::Value = resp.json().await.unwrap();
    let workflow_id = as_i64(&workflow, "id");

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows/{}/statuses", base_url(), workflow_id))
        .json(&json!({ "key": "todo", "label": "To Do", "category": "todo", "position": 0, "is_initial": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let status_a: serde_json::Value = resp.json().await.unwrap();
    let status_a_id = as_i64(&status_a, "id");

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows/{}/statuses", base_url(), workflow_id))
        .json(&json!({ "key": "done", "label": "Done", "category": "done", "position": 1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let status_b: serde_json::Value = resp.json().await.unwrap();
    let status_b_id = as_i64(&status_b, "id");

    // Only todo -> done is declared, not the reverse.
    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows/{}/transitions", base_url(), workflow_id))
        .json(&json!({ "from_status_id": status_a_id, "to_status_id": status_b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = admin
        .client
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({ "name": unique("custom-workflow-project"), "workflow_id": workflow_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let project: serde_json::Value = resp.json().await.unwrap();
    let project_id = as_i64(&project, "id");

    let ticket = admin.create_ticket(project_id, None, "Ticket on custom workflow").await;
    let ticket_id = as_i64(&ticket, "id");
    assert_eq!(ticket["status"]["id"], status_a_id, "new tickets should start on the initial status");

    // Declared transition succeeds.
    let resp = admin
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": status_b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // The reverse transition was never declared, so it must be rejected.
    let resp = admin
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": status_a_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "undeclared transitions must be rejected");

    // Re-setting the same status is always a no-op, never a rejected "transition".
    let resp = admin
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "status_id": status_b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn workflow_mutations_require_admin() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/workflows", base_url()))
        .json(&json!({ "name": unique("should-not-be-created") }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "creating a workflow must require admin");

    // Reads, on the other hand, are open to any authenticated user (needed
    // for the project-creation workflow picker and ticket status dropdowns).
    let resp = user
        .client
        .get(format!("{}/api/v1/workflows", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn workflow_status_and_transition_mutations_are_recorded_in_the_audit_log() {
    let admin = TestUser::create_admin().await;

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows", base_url()))
        .json(&json!({ "name": unique("audited-workflow") }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let workflow: serde_json::Value = resp.json().await.unwrap();
    let workflow_id = as_i64(&workflow, "id");

    let resp = admin
        .client
        .patch(format!("{}/api/v1/workflows/{}", base_url(), workflow_id))
        .json(&json!({ "name": unique("audited-workflow-renamed") }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows/{}/statuses", base_url(), workflow_id))
        .json(&json!({ "key": "todo", "label": "To Do", "category": "todo", "position": 0, "is_initial": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let status_a: serde_json::Value = resp.json().await.unwrap();
    let status_a_id = as_i64(&status_a, "id");

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows/{}/statuses", base_url(), workflow_id))
        .json(&json!({ "key": "done", "label": "Done", "category": "done", "position": 1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let status_b: serde_json::Value = resp.json().await.unwrap();
    let status_b_id = as_i64(&status_b, "id");

    let resp = admin
        .client
        .patch(format!("{}/api/v1/workflows/{}/statuses/{}", base_url(), workflow_id, status_a_id))
        .json(&json!({ "label": "To Do (renamed)" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows/{}/transitions", base_url(), workflow_id))
        .json(&json!({ "from_status_id": status_a_id, "to_status_id": status_b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let transition: serde_json::Value = resp.json().await.unwrap();
    let transition_id = as_i64(&transition, "id");

    let resp = admin
        .client
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), admin.id))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "workflow_created" && e["target_id"] == workflow_id),
        "creating a workflow must leave an audit_log entry"
    );
    assert!(
        entries.iter().any(|e| e["action"] == "workflow_updated" && e["target_id"] == workflow_id),
        "renaming a workflow must leave an audit_log entry"
    );
    assert!(
        entries.iter().any(|e| e["action"] == "workflow_status_created" && e["target_id"] == status_a_id),
        "creating a workflow status must leave an audit_log entry"
    );
    let status_update = entries
        .iter()
        .find(|e| e["action"] == "workflow_status_updated" && e["target_id"] == status_a_id)
        .expect("updating a workflow status must leave an audit_log entry");
    assert_eq!(status_update["from_text"], "To Do");
    assert_eq!(status_update["to_text"], "To Do (renamed)");
    assert!(
        entries
            .iter()
            .any(|e| e["action"] == "workflow_transition_created" && e["target_id"] == transition_id),
        "creating a workflow transition must leave an audit_log entry"
    );
}

#[tokio::test]
async fn the_default_workflow_cannot_be_deleted_and_an_in_use_workflow_is_protected() {
    let admin = TestUser::create_admin().await;

    let resp = admin
        .client
        .get(format!("{}/api/v1/workflows", base_url()))
        .send()
        .await
        .unwrap();
    let workflows: serde_json::Value = resp.json().await.unwrap();
    let default_workflow_id = workflows
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["is_default"] == true)
        .expect("a default workflow must always exist")["id"]
        .as_i64()
        .unwrap();

    let resp = admin
        .client
        .delete(format!("{}/api/v1/workflows/{}", base_url(), default_workflow_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "the default workflow must never be deletable");

    let resp = admin
        .client
        .post(format!("{}/api/v1/workflows", base_url()))
        .json(&json!({ "name": unique("in-use-workflow") }))
        .send()
        .await
        .unwrap();
    let workflow: serde_json::Value = resp.json().await.unwrap();
    let workflow_id = as_i64(&workflow, "id");
    admin
        .client
        .post(format!("{}/api/v1/workflows/{}/statuses", base_url(), workflow_id))
        .json(&json!({ "key": "only", "label": "Only", "category": "todo", "position": 0, "is_initial": true }))
        .send()
        .await
        .unwrap();

    let resp = admin
        .client
        .post(format!("{}/api/v1/projects", base_url()))
        .json(&json!({ "name": unique("uses-custom-workflow"), "workflow_id": workflow_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = admin
        .client
        .delete(format!("{}/api/v1/workflows/{}", base_url(), workflow_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "a workflow assigned to a project must not be deletable");
}
