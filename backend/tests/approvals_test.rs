mod common;
use common::{base_url, now_jst, TestUser};
use serde_json::json;

async fn submit_a_valid_day(user: &TestUser) -> String {
    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "did some work" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    now_jst().format("%Y-%m-%d").to_string()
}

async fn decide(approver: &TestUser, subject_id: i64, date: &str, decision: &str, comment: &str) -> reqwest::Response {
    approver
        .client
        .post(format!(
            "{}/api/v1/approvals/subordinates/{}/day/{}/decide",
            base_url(),
            subject_id,
            date
        ))
        .json(&json!({ "decision": decision, "comment": comment }))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn deciding_requires_being_an_assigned_approver() {
    let subject = TestUser::create().await;
    let outsider = TestUser::create().await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&outsider, subject.id, &date, "approved", "looks fine").await;
    assert_eq!(resp.status(), 403, "a user with no approver assignment must not be able to decide");
}

#[tokio::test]
async fn approval_is_recorded_with_who_when_and_comment() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&approver, subject.id, &date, "approved", "great work today").await;
    assert_eq!(resp.status(), 200);
    let record: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(record["decision"], "approved");
    assert_eq!(record["comment"], "great work today");
    assert_eq!(record["approver_id"], approver.id);
    assert!(!record["decided_at"].as_str().unwrap().is_empty());

    let resp = approver
        .client
        .get(format!(
            "{}/api/v1/approvals/subordinates/{}/day/{}",
            base_url(),
            subject.id,
            date
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "approved");
    let history = body["history"].as_array().unwrap();
    assert_eq!(history.len(), 1, "the decision must show up in the day's audit history");
    assert_eq!(history[0]["decision"], "approved");
    assert_eq!(history[0]["approver_id"], approver.id);
}

#[tokio::test]
async fn top_level_user_can_self_approve() {
    let user = TestUser::create().await;
    user.become_approver_of(&user).await;
    let date = submit_a_valid_day(&user).await;

    let resp = decide(&user, user.id, &date, "approved", "self-approved").await;
    assert_eq!(resp.status(), 200, "a user assigned as their own approver must be able to decide on their own day");
}

#[tokio::test]
async fn deciding_before_a_report_is_submitted_is_rejected() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;

    subject
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    subject
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    // No report submitted yet - the day is "invalid", not "valid".

    let today = now_jst().format("%Y-%m-%d").to_string();
    let resp = decide(&approver, subject.id, &today, "approved", "too early").await;
    assert_eq!(resp.status(), 400, "a day with no submitted report cannot be decided on");
}

#[tokio::test]
async fn rejection_keeps_the_report_editable_and_resubmitting_clears_it() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&approver, subject.id, &date, "rejected", "please add more detail").await;
    assert_eq!(resp.status(), 200);

    let resp = subject
        .client
        .get(format!("{}/api/v1/daily-reports/{}", base_url(), date))
        .send()
        .await
        .unwrap();
    let view: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(view["locked"], false, "a rejected day must remain editable");

    // Resubmitting must clear the rejection - the day goes back to "valid".
    let resp = subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "added more detail" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = subject
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    let today: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(today["status"], "valid", "resubmitting after a rejection must clear it back to awaiting decision");

    // The approver can now decide on it again.
    let resp = decide(&approver, subject.id, &date, "approved", "good now").await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn approved_day_locks_the_report_from_further_edits() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&approver, subject.id, &date, "approved", "approved").await;
    assert_eq!(resp.status(), 200);

    let resp = subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "trying to sneak in an edit" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "an approved day's report must not be editable");

    let resp = subject
        .client
        .get(format!("{}/api/v1/daily-reports/{}", base_url(), date))
        .send()
        .await
        .unwrap();
    let view: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(view["locked"], true);
    assert_eq!(view["lock_reason"], "approved");
}

#[tokio::test]
async fn closing_the_month_requires_every_day_to_be_approved_first() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    submit_a_valid_day(&subject).await;

    let now = now_jst();
    let (year, month) = (now.format("%Y").to_string(), now.format("%m").to_string());

    let resp = approver
        .client
        .post(format!(
            "{}/api/v1/approvals/subordinates/{}/month/{}/{}/close",
            base_url(),
            subject.id,
            year,
            month
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "a month with a not-yet-approved day must not be closeable");
}

#[tokio::test]
async fn full_close_then_lock_then_reopen_cycle() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&approver, subject.id, &date, "approved", "all good").await;
    assert_eq!(resp.status(), 200);

    let now = now_jst();
    let (year, month) = (now.format("%Y").to_string(), now.format("%m").to_string());
    let close_url = format!(
        "{}/api/v1/approvals/subordinates/{}/month/{}/{}/close",
        base_url(),
        subject.id,
        year,
        month
    );
    let reopen_url = format!(
        "{}/api/v1/approvals/subordinates/{}/month/{}/{}/reopen",
        base_url(),
        subject.id,
        year,
        month
    );

    let resp = approver.client.post(&close_url).send().await.unwrap();
    assert_eq!(resp.status(), 200, "every day is approved, so the month must be closeable");
    let closure: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(closure["closed"], true);
    assert_eq!(closure["closed_by"], approver.id);

    // Closed: neither the report nor a fresh work session can be touched.
    let resp = subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "edit after close" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "a closed month's report must not be editable");

    let resp = subject
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "starting a new session in a closed month must be rejected");

    // Reopen: daily approvals in that month must revert to "valid" so the
    // report becomes editable and re-approvable again.
    let resp = approver.client.post(&reopen_url).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let closure: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(closure["closed"], false);

    let resp = approver
        .client
        .get(format!(
            "{}/api/v1/approvals/subordinates/{}/day/{}",
            base_url(),
            subject.id,
            date
        ))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "valid", "reopening the month must revert the day's approval back to awaiting decision");

    let resp = subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "fixed after reopen" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "after a reopen the report must be editable again");

    let resp = decide(&approver, subject.id, &date, "approved", "re-approved").await;
    assert_eq!(resp.status(), 200);

    let resp = approver.client.post(&close_url).send().await.unwrap();
    assert_eq!(resp.status(), 200, "the month must be closeable again after the redo cycle");
}

#[tokio::test]
async fn decision_history_preserves_every_past_decision_for_audit() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&approver, subject.id, &date, "rejected", "first pass: needs work").await;
    assert_eq!(resp.status(), 200);

    subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "revised" }))
        .send()
        .await
        .unwrap();

    let resp = decide(&approver, subject.id, &date, "approved", "second pass: good").await;
    assert_eq!(resp.status(), 200);

    let resp = approver
        .client
        .get(format!(
            "{}/api/v1/approvals/subordinates/{}/day/{}",
            base_url(),
            subject.id,
            date
        ))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let history = body["history"].as_array().unwrap();
    assert_eq!(history.len(), 2, "both the superseded rejection and the current approval must remain in history");
    // Newest first.
    assert_eq!(history[0]["decision"], "approved");
    assert_eq!(history[0]["comment"], "second pass: good");
    assert_eq!(history[1]["decision"], "rejected");
    assert_eq!(history[1]["comment"], "first pass: needs work");
}

#[tokio::test]
async fn an_approved_day_can_be_revoked_by_rejecting_it() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    let resp = decide(&approver, subject.id, &date, "approved", "approved by mistake").await;
    assert_eq!(resp.status(), 200);

    // Approving again (a no-op re-approval) must still be rejected.
    let resp = decide(&approver, subject.id, &date, "approved", "again").await;
    assert_eq!(resp.status(), 400, "an already-approved day cannot simply be re-approved");

    // But rejecting it acts as a revoke.
    let resp = decide(&approver, subject.id, &date, "rejected", "oops, revoking this").await;
    assert_eq!(resp.status(), 200, "an approved day must be revocable by rejecting it");

    let resp = subject
        .client
        .get(format!("{}/api/v1/daily-reports/{}", base_url(), date))
        .send()
        .await
        .unwrap();
    let view: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(view["locked"], false, "revoking the approval must unlock the report again");

    let resp = approver
        .client
        .get(format!(
            "{}/api/v1/approvals/subordinates/{}/day/{}",
            base_url(),
            subject.id,
            date
        ))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "rejected");
    let history = body["history"].as_array().unwrap();
    assert_eq!(history.len(), 2, "both the original approval and the revoke must be kept for audit");
    assert_eq!(history[0]["decision"], "rejected");
    assert_eq!(history[0]["comment"], "oops, revoking this");
    assert_eq!(history[1]["decision"], "approved");
}

#[tokio::test]
async fn revoking_is_blocked_once_the_month_is_closed() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;
    let date = submit_a_valid_day(&subject).await;

    decide(&approver, subject.id, &date, "approved", "approved").await;

    let now = now_jst();
    let (year, month) = (now.format("%Y").to_string(), now.format("%m").to_string());
    let resp = approver
        .client
        .post(format!(
            "{}/api/v1/approvals/subordinates/{}/month/{}/{}/close",
            base_url(),
            subject.id,
            year,
            month
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = decide(&approver, subject.id, &date, "rejected", "trying to revoke a closed month").await;
    assert_eq!(resp.status(), 400, "revoking an approval must not be possible while the month is closed");
}

#[tokio::test]
async fn subordinates_list_reports_this_months_pending_count() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;

    let resp = approver
        .client
        .get(format!("{}/api/v1/approvals/subordinates", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: serde_json::Value = resp.json().await.unwrap();
    let entry = list.as_array().unwrap().iter().find(|u| u["id"] == subject.id).unwrap();
    assert_eq!(entry["pending_count"], 0, "no submitted report yet means nothing pending");

    let date = submit_a_valid_day(&subject).await;

    let resp = approver
        .client
        .get(format!("{}/api/v1/approvals/subordinates", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    let entry = list.as_array().unwrap().iter().find(|u| u["id"] == subject.id).unwrap();
    assert_eq!(entry["pending_count"], 1, "a submitted, undecided day must count as pending");

    let resp = decide(&approver, subject.id, &date, "approved", "ok").await;
    assert_eq!(resp.status(), 200);

    let resp = approver
        .client
        .get(format!("{}/api/v1/approvals/subordinates", base_url()))
        .send()
        .await
        .unwrap();
    let list: serde_json::Value = resp.json().await.unwrap();
    let entry = list.as_array().unwrap().iter().find(|u| u["id"] == subject.id).unwrap();
    assert_eq!(entry["pending_count"], 0, "a decided day must no longer count as pending");
}

#[tokio::test]
async fn approvals_endpoints_require_authentication() {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/approvals/subordinates", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}
