mod common;
use common::{as_i64, base_url, now_jst, unique, TestUser};
use serde_json::{json, Value};

async fn notifications_for(user: &TestUser) -> Vec<Value> {
    let resp = user
        .client
        .get(format!("{}/api/v1/notifications", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    body["items"].as_array().unwrap().clone()
}

fn find_notification<'a>(items: &'a [Value], notif_type: &str) -> Option<&'a Value> {
    items.iter().find(|n| n["notif_type"] == notif_type)
}

#[tokio::test]
async fn mentioning_a_user_in_a_comment_notifies_them() {
    let author = TestUser::create().await;
    let mentioned = TestUser::create().await;
    let project = author.create_project(&unique("mention-comment-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = author.create_ticket(project_id, None, "Ticket with a mention").await;
    let ticket_id = as_i64(&ticket, "id");

    let body = format!("cc @[Someone](user:{}) please check this", mentioned.id);
    let resp = author
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": body }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let items = notifications_for(&mentioned).await;
    let notif = find_notification(&items, "ticket_mentioned").expect("mentioned user should have a ticket_mentioned notification");
    assert_eq!(notif["ticket_id"], ticket_id);
    assert_eq!(notif["actor_name"], "Test User");
}

#[tokio::test]
async fn mentioning_the_reporter_in_a_comment_does_not_double_notify() {
    // The reporter already gets a `ticket_commented` notification (they have
    // a stake in the ticket) - a third party mentioning them too should not
    // additionally produce a redundant `ticket_mentioned` notification for
    // the same comment.
    let author = TestUser::create().await;
    let project = author.create_project(&unique("mention-self-stake-project")).await;
    let project_id = as_i64(&project, "id");
    let ticket = author.create_ticket(project_id, None, "Ticket").await;
    let ticket_id = as_i64(&ticket, "id");

    let body = format!("cc @[Reporter](user:{}) fyi", author.id);
    let resp = author
        .client
        .post(format!("{}/api/v1/tickets/{}/comments", base_url(), ticket_id))
        .json(&json!({ "body": body }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let items = notifications_for(&author).await;
    assert!(
        find_notification(&items, "ticket_mentioned").is_none(),
        "mentioning the reporter, who is already a stakeholder, must not create a separate ticket_mentioned notification"
    );
    assert!(
        find_notification(&items, "ticket_commented").is_none(),
        "commenting on your own ticket must not notify yourself either"
    );
}

#[tokio::test]
async fn mentioning_a_user_in_ticket_description_on_create_notifies_them() {
    let author = TestUser::create().await;
    let mentioned = TestUser::create().await;
    let project = author.create_project(&unique("mention-create-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = author
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({
            "project_id": project_id,
            "parent_ticket_id": null,
            "assignee_id": null,
            "title": "Ticket with mention in description",
            "description": format!("please review @[Someone](user:{})", mentioned.id),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let ticket: Value = resp.json().await.unwrap();
    let ticket_id = as_i64(&ticket, "id");

    let items = notifications_for(&mentioned).await;
    let notif = find_notification(&items, "ticket_mentioned").expect("mentioned user should be notified on ticket creation");
    assert_eq!(notif["ticket_id"], ticket_id);
}

fn count_ticket_mentioned(items: &[Value]) -> usize {
    items.iter().filter(|n| n["notif_type"] == "ticket_mentioned").count()
}

#[tokio::test]
async fn editing_ticket_description_only_notifies_newly_added_mentions() {
    let author = TestUser::create().await;
    let already_mentioned = TestUser::create().await;
    let newly_mentioned = TestUser::create().await;
    let project = author.create_project(&unique("mention-edit-project")).await;
    let project_id = as_i64(&project, "id");

    // Created with no mentions at all, so `already_mentioned`'s one and only
    // notification below is unambiguously caused by the first edit.
    let resp = author
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({
            "project_id": project_id,
            "parent_ticket_id": null,
            "assignee_id": null,
            "title": "Ticket",
            "description": "",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let ticket: Value = resp.json().await.unwrap();
    let ticket_id = as_i64(&ticket, "id");

    // First edit: introduces the "A" mention.
    let resp = author
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({ "description": format!("@[A](user:{})", already_mentioned.id) }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(count_ticket_mentioned(&notifications_for(&already_mentioned).await), 1);

    // Second edit: keeps "A", adds "B" - only B is new.
    let resp = author
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({
            "description": format!("@[A](user:{}) and @[B](user:{})", already_mentioned.id, newly_mentioned.id),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        count_ticket_mentioned(&notifications_for(&already_mentioned).await),
        1,
        "a mention that was already present before the edit must not be re-notified"
    );
    assert_eq!(count_ticket_mentioned(&notifications_for(&newly_mentioned).await), 1, "the newly added mention must be notified");

    // Third edit: no textual change to the mention set - re-saving the same
    // description must not notify again.
    let resp = author
        .client
        .patch(format!("{}/api/v1/tickets/{}", base_url(), ticket_id))
        .json(&json!({
            "description": format!("@[A](user:{}) and @[B](user:{})", already_mentioned.id, newly_mentioned.id),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        count_ticket_mentioned(&notifications_for(&newly_mentioned).await),
        1,
        "an unchanged mention set must not be re-notified on a no-op edit"
    );
}

#[tokio::test]
async fn mentioning_a_user_in_a_daily_report_notifies_them_without_a_link() {
    let author = TestUser::create().await;
    let mentioned = TestUser::create().await;
    let date = now_jst().date_naive();

    let resp = author
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({
            "report_date": date,
            "achievement": format!("worked with @[Someone](user:{})", mentioned.id),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let items = notifications_for(&mentioned).await;
    let notif = find_notification(&items, "report_mentioned").expect("mentioned user should have a report_mentioned notification");
    assert_eq!(notif["report_date"], date.to_string());
    assert_eq!(notif["ticket_id"], Value::Null, "report mentions must not carry a ticket_id (no ticket-view link)");
}
