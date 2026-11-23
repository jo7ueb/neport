mod common;
use common::{as_i64, base_url, unique, TestUser};
use serde_json::json;

#[tokio::test]
async fn transitive_dependency_cycle_is_rejected() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("dependency-project")).await;
    let project_id = as_i64(&project, "id");

    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, None, "B").await;
    let b_id = as_i64(&b, "id");
    let c = user.create_ticket(project_id, None, "C").await;
    let c_id = as_i64(&c, "id");

    // A -> B
    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // B -> C
    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), b_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": c_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // C -> A would close the loop.
    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), c_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": a_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let deps: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(deps["successors"].as_array().unwrap().len(), 1);
    assert_eq!(deps["successors"][0]["id"], b_id);
    assert!(
        deps["successors"][0]["dependency_id"].as_i64().is_some(),
        "each entry must include the dependency row's own id so the frontend can delete it"
    );
}

#[tokio::test]
async fn a_ticket_cannot_depend_on_itself() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("dependency-self-project")).await;
    let project_id = as_i64(&project, "id");
    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": a_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn dependency_can_be_deleted() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("dependency-delete-project")).await;
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
    let dep: serde_json::Value = resp.json().await.unwrap();
    let dep_id = as_i64(&dep, "id");

    let resp = user
        .client
        .delete(format!(
            "{}/api/v1/tickets/{}/dependencies/{}",
            base_url(),
            a_id,
            dep_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Removing a dependency is a soft delete (see tickets::dependencies) so
    // the ticket timeline can still show "X added/removed this dependency"
    // afterward - the row keeps appearing in this list, just flagged via
    // dependency_deleted_at, rather than disappearing outright.
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .send()
        .await
        .unwrap();
    let deps: serde_json::Value = resp.json().await.unwrap();
    let successors = deps["successors"].as_array().unwrap();
    assert_eq!(successors.len(), 1);
    assert!(!successors[0]["dependency_deleted_at"].is_null());
}

#[tokio::test]
async fn a_dependency_cannot_be_removed_through_an_unrelated_ticket() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("dependency-unrelated-delete")).await;
    let project_id = as_i64(&project, "id");
    let a = user.create_ticket(project_id, None, "A").await;
    let a_id = as_i64(&a, "id");
    let b = user.create_ticket(project_id, None, "B").await;
    let b_id = as_i64(&b, "id");
    let c = user.create_ticket(project_id, None, "C").await;
    let c_id = as_i64(&c, "id");

    // A -> B, and C is an unrelated ticket in the same project.
    let resp = user
        .client
        .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .json(&json!({ "direction": "successor", "other_ticket_id": b_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let dep: serde_json::Value = resp.json().await.unwrap();
    let dep_id = as_i64(&dep, "id");

    // Deleting A->B via C's path must be refused: previously this removed
    // any dependency the caller happened to know the id of.
    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}/dependencies/{}", base_url(), c_id, dep_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "an unrelated ticket must not be able to remove this dependency");

    // A->B still exists and is still active.
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .send()
        .await
        .unwrap();
    let deps: serde_json::Value = resp.json().await.unwrap();
    let successors = deps["successors"].as_array().unwrap();
    assert_eq!(successors.len(), 1);
    assert!(
        successors[0]["dependency_deleted_at"].is_null(),
        "the dependency must remain active after the rejected delete"
    );

    // The link can still be removed through either of its own end tickets.
    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}/dependencies/{}", base_url(), b_id, dep_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204, "deleting through the successor end must work");

    // Deleting again is a no-op success, not an error (keeps retries simple).
    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}/dependencies/{}", base_url(), a_id, dep_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204, "re-deleting an already removed dependency must not fail");

    // A dependency id that does not exist is a 404, not a silent success.
    let resp = user
        .client
        .delete(format!("{}/api/v1/tickets/{}/dependencies/{}", base_url(), a_id, 999_999_999_i64))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn a_dependency_cannot_link_tickets_across_projects() {
    let user = TestUser::create().await;
    let own_project = user.create_project(&unique("dependency-cross-own")).await;
    let own_project_id = as_i64(&own_project, "id");
    let other_project = user.create_project(&unique("dependency-cross-other")).await;
    let other_project_id = as_i64(&other_project, "id");

    let own = user.create_ticket(own_project_id, None, "Own").await;
    let own_id = as_i64(&own, "id");
    let other = user.create_ticket(other_project_id, None, "Other").await;
    let other_id = as_i64(&other, "id");

    for direction in ["successor", "predecessor"] {
        let resp = user
            .client
            .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), own_id))
            .json(&json!({ "direction": direction, "other_ticket_id": other_id }))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            400,
            "direction={} must not link tickets in different projects",
            direction
        );
    }

    // Neither rejection left a row behind on the own ticket.
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/dependencies", base_url(), own_id))
        .send()
        .await
        .unwrap();
    let deps: serde_json::Value = resp.json().await.unwrap();
    assert!(deps["successors"].as_array().unwrap().is_empty());
    assert!(deps["predecessors"].as_array().unwrap().is_empty());

    // Same-project links still work in both directions.
    let third = user.create_ticket(own_project_id, None, "Third").await;
    let third_id = as_i64(&third, "id");
    for direction in ["successor", "predecessor"] {
        let resp = user
            .client
            .post(format!("{}/api/v1/tickets/{}/dependencies", base_url(), own_id))
            .json(&json!({ "direction": direction, "other_ticket_id": third_id }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "direction={} within one project must be accepted", direction);
    }
}

#[tokio::test]
async fn dependency_can_be_deleted_using_the_dependency_id_from_the_list_endpoint() {
    let user = TestUser::create().await;
    let project = user.create_project(&unique("dependency-list-delete-project")).await;
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

    // The frontend only ever has the list response to work with (it never
    // sees the create response), so it must be able to delete using the
    // dependency_id it finds there.
    let resp = user
        .client
        .get(format!("{}/api/v1/tickets/{}/dependencies", base_url(), a_id))
        .send()
        .await
        .unwrap();
    let deps: serde_json::Value = resp.json().await.unwrap();
    let dependency_id = as_i64(&deps["successors"][0], "dependency_id");

    let resp = user
        .client
        .delete(format!(
            "{}/api/v1/tickets/{}/dependencies/{}",
            base_url(),
            a_id,
            dependency_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}
