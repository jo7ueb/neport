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
            "story_points": 5,
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
    assert_eq!(ticket["story_points"], 5);
    assert_eq!(ticket["assignee_id"], assignee.id);
    assert_eq!(ticket["due_date"], "2026-12-31");
    assert_eq!(ticket["milestone_id"], milestone_id);
}

#[tokio::test]
async fn ticket_story_points_can_be_set_cleared_and_invalid_values_are_rejected() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-story-points")).await;
    let project_id = as_i64(&project, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({
            "project_id": project_id,
            "title": "Estimated ticket",
            "story_points": 3,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let ticket: serde_json::Value = resp.json().await.unwrap();
    let ticket_id = as_i64(&ticket, "id");
    assert_eq!(ticket["story_points"], 3);

    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "story_points": null }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert!(updated["story_points"].is_null());

    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "story_points": 4 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
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
async fn ticket_progress_counts_the_entire_subtree_not_just_direct_children() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("child-progress-project")).await;
    let project_id = as_i64(&project, "id");

    let parent = user.create_ticket(project_id, None, "Parent").await;
    let parent_id = as_i64(&parent, "id");
    assert_eq!(parent["child_ticket_count"], 0, "a childless ticket starts with no children");
    assert_eq!(parent["child_done_ticket_count"], 0);

    let child_a = user.create_ticket(project_id, Some(parent_id), "Child A").await;
    let child_a_id = as_i64(&child_a, "id");
    let child_b = user.create_ticket(project_id, Some(parent_id), "Child B").await;
    let child_b_id = as_i64(&child_b, "id");

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .send()
        .await
        .unwrap();
    let parent: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(parent["child_ticket_count"], 2, "two direct children now exist");
    assert_eq!(parent["child_done_ticket_count"], 0, "neither child is done yet");

    // The workflow is a ladder, so open -> done must pass through in_progress.
    let in_progress_status_id = user.status_id(project_id, "in_progress").await;
    let done_status_id = user.status_id(project_id, "done").await;
    user.client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), child_a_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), child_a_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .send()
        .await
        .unwrap();
    let parent: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(parent["child_ticket_count"], 2);
    assert_eq!(parent["child_done_ticket_count"], 1, "only child A is done");

    // A grandchild (child of child A) must count toward the *parent's*
    // subtree too, not just child A's - this is the whole point of counting
    // recursively instead of only direct children.
    let grandchild = user.create_ticket(project_id, Some(child_a_id), "Grandchild").await;
    let grandchild_id = as_i64(&grandchild, "id");

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .send()
        .await
        .unwrap();
    let parent: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(parent["child_ticket_count"], 3, "the grandchild must count toward the root parent's subtree");
    assert_eq!(parent["child_done_ticket_count"], 1, "the new grandchild is open, not done");

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), child_a_id))
        .send()
        .await
        .unwrap();
    let child_a_fetched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(child_a_fetched["child_ticket_count"], 1, "child A's own subtree is just the grandchild");
    assert_eq!(child_a_fetched["child_done_ticket_count"], 0);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), grandchild_id))
        .send()
        .await
        .unwrap();
    let grandchild_fetched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(grandchild_fetched["child_ticket_count"], 0, "a leaf ticket has no subtree of its own");

    // Deleting the still-open child must drop the total, not the done count.
    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}", base_url(), child_b_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .send()
        .await
        .unwrap();
    let parent: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        parent["child_ticket_count"], 2,
        "the deleted child must no longer be counted, but the surviving grandchild still is"
    );
    assert_eq!(parent["child_done_ticket_count"], 1);
}

#[tokio::test]
async fn a_ticket_cannot_be_closed_while_any_ticket_in_its_subtree_is_still_incomplete() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("close-guard-project")).await;
    let project_id = as_i64(&project, "id");

    let parent = user.create_ticket(project_id, None, "Parent").await;
    let parent_id = as_i64(&parent, "id");
    let child = user.create_ticket(project_id, Some(parent_id), "Child").await;
    let child_id = as_i64(&child, "id");
    let grandchild = user.create_ticket(project_id, Some(child_id), "Grandchild").await;
    let grandchild_id = as_i64(&grandchild, "id");

    let in_progress_status_id = user.status_id(project_id, "in_progress").await;
    let done_status_id = user.status_id(project_id, "done").await;

    // Moving the parent to in_progress is unaffected - only closing (moving
    // into a "done"-category status) is guarded.
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // The grandchild (not just the direct child) is still open, so closing
    // the parent must be rejected.
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "the parent must not be closeable while the grandchild is still open");

    // Finish the grandchild first...
    user.client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), grandchild_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    user.client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), grandchild_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();

    // ...the direct child is still open, so the parent is still blocked.
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "the direct child is still open, so the parent must still be blocked");

    // Finish the direct child too...
    user.client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), child_id))
        .json(&json!({ "status_id": in_progress_status_id }))
        .send()
        .await
        .unwrap();
    user.client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), child_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();

    // ...now every ticket in the subtree is done, so the parent can close.
    let resp = user
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), parent_id))
        .json(&json!({ "status_id": done_status_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "every descendant is now done, so closing the parent must succeed");
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
async fn sort_param_controls_ordering_of_the_ticket_list() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("ticket-sort-project")).await;
    let project_id = as_i64(&project, "id");

    let earlier = user.create_ticket(project_id, None, "Created first").await;
    let earlier_id = as_i64(&earlier, "id");
    let later = user.create_ticket(project_id, None, "Created second").await;
    let later_id = as_i64(&later, "id");

    async fn ids_in_order(user: &TestUser, project_id: i64, sort: &str) -> Vec<i64> {
        let resp = user
            .client
            .get(format!("{}/api/v1/projects/{}/tickets?sort={}", base_url(), project_id, sort))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let list: serde_json::Value = resp.json().await.unwrap();
        list["items"].as_array().unwrap().iter().map(|t| t["id"].as_i64().unwrap()).collect()
    }

    // Default (no sort param) matches created_at_desc, the pre-existing
    // fixed ORDER BY this endpoint always used before sorting was added.
    assert_eq!(ids_in_order(&user, project_id, "created_at_desc").await, vec![later_id, earlier_id]);
    assert_eq!(ids_in_order(&user, project_id, "created_at_asc").await, vec![earlier_id, later_id]);
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

#[tokio::test]
async fn parent_and_milestone_must_belong_to_the_tickets_project() {
    let user = TestUser::create().await;
    let project_id = as_i64(&user.create_project(&unique("ticket-own-project")).await, "id");
    let other_id = as_i64(&user.create_project(&unique("ticket-other-project")).await, "id");
    let parent = user.create_ticket(other_id, None, "Other parent").await;
    let milestone: serde_json::Value = user
        .client
        .post(format!("{}/api/v1/projects/{other_id}/milestones", base_url()))
        .json(&json!({ "name": "Other milestone" }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let ticket_id = as_i64(&user.create_ticket(project_id, None, "Original").await, "id");
    let tickets_url = format!("{}/api/v1/tickets", base_url());
    let ticket_url = format!("{tickets_url}/{ticket_id}");

    for (field, value) in [("parent_ticket_id", parent["id"].clone()), ("milestone_id", milestone["id"].clone())] {
        let create = user.client.post(&tickets_url)
            .json(&json!({ "project_id": project_id, "title": "Rejected", (field): value }))
            .send().await.unwrap();
        assert_eq!(create.status(), 400, "create must reject {field}");
        let update = user.client.patch(&ticket_url)
            .json(&json!({ "title": "Rejected", (field): value }))
            .send().await.unwrap();
        assert_eq!(update.status(), 400, "update must reject {field}");
    }
    let current: serde_json::Value = user.client.get(&ticket_url).send().await.unwrap().json().await.unwrap();
    assert_eq!(current["title"], "Original");
    assert!(current["parent_ticket_id"].is_null());
    assert!(current["milestone_id"].is_null());
}

#[tokio::test]
async fn overlapping_ticket_edits_with_the_same_version_have_one_winner() {
    let user = TestUser::create().await;
    let project_id = as_i64(&user.create_project(&unique("ticket-conflict")).await, "id");
    let ticket = user.create_ticket(project_id, None, "Original").await;
    let id = as_i64(&ticket, "id");
    let url = format!("{}/api/v1/tickets/{id}", base_url());
    let left = user.client.patch(&url)
        .json(&json!({ "title": "Left", "expected_updated_at": ticket["updated_at"] }));
    let right = user.client.patch(&url)
        .json(&json!({ "title": "Right", "expected_updated_at": ticket["updated_at"] }));
    let (left, right) = tokio::join!(left.send(), right.send());
    let left = left.unwrap();
    let right = right.unwrap();
    let mut statuses = [left.status().as_u16(), right.status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 409]);
    let winner = if left.status() == 200 { "Left" } else { "Right" };
    let activity: serde_json::Value = user.client.get(format!("{url}/activity"))
        .send().await.unwrap().json().await.unwrap();
    let title_changes: Vec<_> = activity["items"].as_array().unwrap().iter()
        .filter(|row| row["field_name"] == "title").collect();
    assert_eq!(title_changes.len(), 1);
    assert_eq!(title_changes[0]["from_text"], "Original");
    assert_eq!(title_changes[0]["to_text"], winner);
    let stored: serde_json::Value = user.client.get(&url).send().await.unwrap().json().await.unwrap();
    assert_eq!(stored["title"], winner);
    assert_ne!(stored["updated_at"], ticket["updated_at"]);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to the disposable database used by the test API"]
async fn activity_write_failure_rolls_back_ticket_and_audit() {
    let user = TestUser::create().await;
    let project_id = as_i64(&user.create_project(&unique("ticket-history-failure")).await, "id");
    let ticket = user.create_ticket(project_id, None, "Original").await;
    let id = as_i64(&ticket, "id");
    let url = format!("{}/api/v1/tickets/{id}", base_url());
    let db = sqlx::PgPool::connect(&std::env::var("TEST_DATABASE_URL").unwrap()).await.unwrap();
    // This constraint is specific to the test ticket and fails its second history insert.
    let constraint = format!("test_history_failure_{id}");
    sqlx::query(&format!(
        "ALTER TABLE ticket_activity_log ADD CONSTRAINT {constraint}
         CHECK (ticket_id <> {id} OR field_name IS DISTINCT FROM 'description') NOT VALID"
    )).execute(&db).await.unwrap();
    let access_count = || sqlx::query_scalar::<_, i64>("SELECT count(*) FROM access_log WHERE actor_id = $1")
        .bind(user.id)
        .fetch_one(&db);
    let count_before = access_count().await;
    let response = user.client.patch(&url)
        .json(&json!({ "title": "Updated", "description": "New description" }))
        .send().await;
    sqlx::query(&format!("ALTER TABLE ticket_activity_log DROP CONSTRAINT {constraint}"))
        .execute(&db).await.unwrap();

    assert_eq!(response.unwrap().status(), 500);
    assert_eq!(access_count().await.unwrap(), count_before.unwrap());
    let stored: serde_json::Value = user.client.get(&url).send().await.unwrap().json().await.unwrap();
    assert_eq!(stored, ticket);
    let activity: serde_json::Value = user.client.get(format!("{url}/activity"))
        .send().await.unwrap().json().await.unwrap();
    let entries = activity["items"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["activity_type"], "created");
}
