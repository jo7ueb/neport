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
