mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn ticket_can_be_created_with_full_attributes_in_one_request() {
    let user = TestUser::create().await;
    let assignee = TestUser::create().await;
    let project = user.create_project(&unique("ticket-attrs-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .post(format!(
            "{}/api/v1/projects/{}/milestones",
            base_url(),
            project_id
        ))
        .json(&json!({ "name": "v1.0" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let milestone: serde_json::Value = resp.json().await.unwrap();
    let milestone_id = as_i64(&milestone, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({
            "project_id": project_id,
            "title": "Fully specified ticket",
            "description": "some details",
            "assignee_id": assignee.id,
            "due_date": "2026-12-31",
            "milestone_id": milestone_id,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "ticket creation should accept all attributes in one request");
    let ticket: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(ticket["description"], "some details");
    assert_eq!(ticket["assignee_id"], assignee.id);
    assert_eq!(ticket["due_date"], "2026-12-31");
    assert_eq!(ticket["milestone_id"], milestone_id);
}

#[tokio::test]
async fn create_get_update_and_delete_ticket() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-crud-project")).await;
    let project_id = as_i64(&project, "id");

    let ticket = user.create_ticket(project_id, None, "Ticket A").await;
    let ticket_id = as_i64(&ticket, "id");
    assert_eq!(ticket["reporter_id"], user.id, "reporter_id must come from the session, not the request body");
    assert_eq!(ticket["status"]["key"], "open");

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

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
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(updated["status"]["key"], "done");

    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn deleting_a_parent_ticket_cascades_to_children_and_dependencies() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-cascade-project")).await;
    let project_id = as_i64(&project, "id");

    let parent = user.create_ticket(project_id, None, "Parent").await;
    let parent_id = as_i64(&parent, "id");
    let child = user.create_ticket(project_id, Some(parent_id), "Child").await;
    let child_id = as_i64(&child, "id");
    let other = user.create_ticket(project_id, None, "Other").await;
    let other_id = as_i64(&other, "id");

    // Parent depends on `other`, and has a comment.
    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), parent_id))
        .json(&json!({ "direction": "predecessor", "other_ticket_id": other_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), parent_id))
        .json(&json!({ "body": "a comment on the parent" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), child_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "the child ticket must be cascade-deleted with its parent");

    // `other` itself must survive (only its dependency row on the deleted
    // parent should be cascade-removed).
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), other_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "unrelated tickets must not be affected by the cascade");
}

#[tokio::test]
async fn reparenting_a_ticket_into_its_own_descendant_is_rejected_as_a_cycle() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-cycle-project")).await;
    let project_id = as_i64(&project, "id");

    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, Some(a_id), "B").await;
    let b_id = as_i64(&b, "id");
    let c = user.create_ticket(project_id, Some(b_id), "C").await;
    let c_id = as_i64(&c, "id");

    // A -> B -> C exists; moving A under C would create a cycle.
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), a_id))
        .json(&json!({ "parent_ticket_id": c_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), a_id))
        .json(&json!({ "parent_ticket_id": a_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "a ticket cannot become its own parent");
}

#[tokio::test]
async fn list_children_returns_only_direct_children() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-children-project")).await;
    let project_id = as_i64(&project, "id");

    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, Some(a_id), "B").await;
    let b_id = as_i64(&b, "id");
    user.create_ticket(project_id, Some(b_id), "C").await;

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/children", base_url(), a_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let children: serde_json::Value = resp.json().await.unwrap();
    let children = children.as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], b_id);
}

#[tokio::test]
async fn project_scoped_list_and_tree_endpoints_return_the_hierarchy() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-tree-project")).await;
    let project_id = as_i64(&project, "id");

    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, Some(a_id), "B").await;
    let b_id = as_i64(&b, "id");

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}/tickets", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(list["items"].as_array().unwrap().len(), 2);
    assert_eq!(list["total"], 2);

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}/tickets/tree", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let tree: serde_json::Value = resp.json().await.unwrap();
    let roots = tree.as_array().unwrap();
    assert_eq!(roots.len(), 1, "only the root ticket should appear at the top level");
    assert_eq!(roots[0]["id"], a_id);
    let children = roots[0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], b_id);
}

#[tokio::test]
async fn tree_endpoint_preserves_multi_level_descendants() {
    // Regression test: the tree-building code used to walk a HashMap in
    // unspecified order, which could detach a subtree whenever a middle
    // node was linked to its own parent before its children were linked to
    // it. A chain several levels deep, plus branching siblings, exercises
    // that ordering-dependent path.
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-deep-tree-project")).await;
    let project_id = as_i64(&project, "id");

    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, Some(a_id), "B").await;
    let b_id = as_i64(&b, "id");
    let c = user.create_ticket(project_id, Some(b_id), "C").await;
    let c_id = as_i64(&c, "id");
    let d = user.create_ticket(project_id, Some(c_id), "D").await;
    let d_id = as_i64(&d, "id");
    // A sibling branch under B, so B has two children at once.
    let e = user.create_ticket(project_id, Some(b_id), "E").await;
    let e_id = as_i64(&e, "id");

    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}/tickets/tree", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let tree: serde_json::Value = resp.json().await.unwrap();
    let roots = tree.as_array().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["id"], a_id);

    let a_children = roots[0]["children"].as_array().unwrap();
    assert_eq!(a_children.len(), 1);
    assert_eq!(a_children[0]["id"], b_id);

    let b_children = a_children[0]["children"].as_array().unwrap();
    assert_eq!(b_children.len(), 2, "B must keep both of its children (C and E)");
    let b_children_ids: Vec<i64> = b_children.iter().map(|n| n["id"].as_i64().unwrap()).collect();
    assert!(b_children_ids.contains(&c_id));
    assert!(b_children_ids.contains(&e_id));

    let c_node = b_children.iter().find(|n| n["id"] == c_id).unwrap();
    let c_children = c_node["children"].as_array().unwrap();
    assert_eq!(c_children.len(), 1, "D must not be dropped from the tree");
    assert_eq!(c_children[0]["id"], d_id);
}

#[tokio::test]
async fn overdue_filter_only_returns_open_tickets_past_due_date() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-overdue-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({
            "project_id": project_id,
            "parent_ticket_id": null,
            "assignee_id": null,
            "title": "Overdue ticket",
            "due_date": "2000-01-01",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let overdue: serde_json::Value = resp.json().await.unwrap();
    let overdue_id = as_i64(&overdue, "id");

    user.create_ticket(project_id, None, "Not overdue").await;

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/tickets?overdue=true",
            base_url(),
            project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: serde_json::Value = resp.json().await.unwrap();
    let ids: Vec<i64> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![overdue_id]);
}

#[tokio::test]
async fn ticket_list_is_paginated() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-page-project")).await;
    let project_id = as_i64(&project, "id");

    for i in 0..3 {
        user.create_ticket(project_id, None, &format!("Ticket {i}")).await;
    }

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/tickets?page=1&per_page=2",
            base_url(),
            project_id
        ))
        .send()
        .await
        .unwrap();
    let page1: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(page1["total"], 3);
    assert_eq!(page1["items"].as_array().unwrap().len(), 2);

    let resp = user
        .client
        .get(format!(
            "{}/api/v1/projects/{}/tickets?page=2&per_page=2",
            base_url(),
            project_id
        ))
        .send()
        .await
        .unwrap();
    let page2: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(page2["items"].as_array().unwrap().len(), 1, "the third ticket should be on the second page");
}
