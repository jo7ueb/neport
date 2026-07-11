mod common;
use chrono::{Duration, Utc};
use common::{admin_client, base_url, now_jst, TestUser};
use serde_json::json;

#[tokio::test]
async fn start_creates_session_and_rejects_double_start() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let session: serde_json::Value = resp.json().await.unwrap();
    assert!(!session["started_at"].is_null());
    assert!(session["ended_at"].is_null());

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "starting work twice without ending must be rejected");

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/current", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn end_requires_an_open_session() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "ending work without starting must be rejected");

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
    let session: serde_json::Value = resp.json().await.unwrap();
    assert!(!session["ended_at"].is_null());

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/current", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "no session should be open after ending");
}

#[tokio::test]
async fn multiple_sessions_in_one_day_are_allowed_without_tracking_breaks() {
    let user = TestUser::create().await;

    // First work period (e.g. morning).
    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    user.client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();

    // Second work period after an untracked break.
    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    user.client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let summary: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(summary["sessions"].as_array().unwrap().len(), 2);
    assert_eq!(summary["status"], "invalid", "no daily report has been submitted yet");
}

#[tokio::test]
async fn day_status_is_in_progress_while_a_session_is_open() {
    let user = TestUser::create().await;

    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let summary: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(summary["status"], "in_progress");
    assert!(summary["last_ended_at"].is_null());
}

#[tokio::test]
async fn day_status_becomes_valid_after_submitting_a_report_without_re_ending() {
    let user = TestUser::create().await;

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
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    let summary: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(summary["status"], "invalid");

    let resp = user
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "submitted after work ended" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Status must be recomputed fresh on read — no explicit "retroactive
    // validation" call is required from the client.
    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    let summary: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(summary["status"], "valid");
}

#[tokio::test]
async fn today_returns_404_when_no_session_started_today() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn monthly_listing_includes_todays_summary() {
    let user = TestUser::create().await;

    user.client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();

    let now = now_jst();
    let resp = user
        .client
        .get(format!(
            "{}/api/v1/work-sessions/monthly?year={}&month={}",
            base_url(),
            now.format("%Y"),
            now.format("%m")
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["days"].as_array().unwrap().len(), 1);
    assert_eq!(body["closure"]["closed"], false);
}

#[tokio::test]
async fn work_session_endpoints_require_authentication() {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/work-sessions/today", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn editing_can_supply_a_forgotten_end_time_and_is_recorded_in_the_audit_log() {
    let user = TestUser::create().await;

    let resp = user
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    let session: serde_json::Value = resp.json().await.unwrap();
    let id = session["id"].as_i64().unwrap();
    assert!(session["ended_at"].is_null());

    // The button-press was forgotten - fix it after the fact instead.
    let ended_at = Utc::now() + Duration::hours(1);
    let resp = user
        .client
        .patch(format!("{}/api/v1/work-sessions/{}", base_url(), id))
        .json(&json!({ "ended_at": ended_at.to_rfc3339() }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert!(!updated["ended_at"].is_null());

    let resp = user
        .client
        .get(format!("{}/api/v1/work-sessions/current", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "the session must no longer be open after the edit");

    let admin = admin_client().await;
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), user.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    let entry = entries
        .iter()
        .find(|e| e["action"] == "work_session_edited" && e["target_id"] == id)
        .expect("editing a session must leave an audit_log entry");
    assert_eq!(entry["actor_id"], user.id);
}

#[tokio::test]
async fn editing_started_at_after_ended_at_is_rejected() {
    let user = TestUser::create().await;

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
    let session: serde_json::Value = resp.json().await.unwrap();
    let id = session["id"].as_i64().unwrap();
    let started_at: chrono::DateTime<Utc> = session["started_at"].as_str().unwrap().parse().unwrap();

    let resp = user
        .client
        .patch(format!("{}/api/v1/work-sessions/{}", base_url(), id))
        .json(&json!({ "ended_at": (started_at - Duration::hours(1)).to_rfc3339() }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "ended_at before started_at must be rejected");
}

#[tokio::test]
async fn editing_someone_elses_session_is_forbidden() {
    let owner = TestUser::create().await;
    let outsider = TestUser::create().await;

    owner
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    let resp = owner
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    let session: serde_json::Value = resp.json().await.unwrap();
    let id = session["id"].as_i64().unwrap();

    let resp = outsider
        .client
        .patch(format!("{}/api/v1/work-sessions/{}", base_url(), id))
        .json(&json!({ "ended_at": Utc::now().to_rfc3339() }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "editing another user's session must be forbidden");
}

#[tokio::test]
async fn editing_a_session_in_a_closed_month_is_rejected() {
    let subject = TestUser::create().await;
    let approver = TestUser::create().await;
    approver.become_approver_of(&subject).await;

    let resp = subject
        .client
        .post(format!("{}/api/v1/work-sessions/start", base_url()))
        .send()
        .await
        .unwrap();
    let session: serde_json::Value = resp.json().await.unwrap();
    let id = session["id"].as_i64().unwrap();
    subject
        .client
        .post(format!("{}/api/v1/work-sessions/end", base_url()))
        .send()
        .await
        .unwrap();
    subject
        .client
        .post(format!("{}/api/v1/daily-reports", base_url()))
        .json(&json!({ "achievement": "did some work" }))
        .send()
        .await
        .unwrap();

    let now = now_jst();
    let date = now.format("%Y-%m-%d").to_string();
    let resp = approver
        .client
        .post(format!(
            "{}/api/v1/approvals/subordinates/{}/day/{}/decide",
            base_url(),
            subject.id,
            date
        ))
        .json(&json!({ "decision": "approved", "comment": "all good" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

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
    assert_eq!(resp.status(), 200, "every day is approved, so the month must be closeable");

    let resp = subject
        .client
        .patch(format!("{}/api/v1/work-sessions/{}", base_url(), id))
        .json(&json!({ "ended_at": Utc::now().to_rfc3339() }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "editing a session in a closed month must be rejected");
}
