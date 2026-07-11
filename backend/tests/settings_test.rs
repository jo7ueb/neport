mod common;
use common::{admin_client, as_i64, base_url, unique, TestUser};
use serde_json::{json, Value};

// system_settings is a single shared system-wide row, and cargo runs the
// separate test binaries (this file among them) one at a time even though
// tests within a binary run concurrently - so keeping every maintenance-mode
// assertion inside one test function (rather than several #[tokio::test]s
// that could interleave) is what keeps the on/off window from racing with
// anything else in this file.
#[tokio::test]
async fn maintenance_mode_blocks_all_writes_except_toggling_itself_back_off() {
    let admin = admin_client().await;
    let user = TestUser::create().await;
    let project = user.create_project(&unique("maintenance-project")).await;
    let project_id = as_i64(&project, "id");

    let resp = admin.get(format!("{}/api/v1/settings", base_url())).send().await.unwrap();
    let original: Value = resp.json().await.unwrap();
    let original_katex = original["katex_enabled"].as_bool().unwrap();
    let original_tz = original["default_timezone"].as_str().unwrap().to_string();

    let set_maintenance = |on: bool| {
        let admin = admin.clone();
        let original_katex = original_katex;
        let original_tz = original_tz.clone();
        async move {
            let resp = admin
                .patch(format!("{}/api/v1/settings", base_url()))
                .json(&json!({ "katex_enabled": original_katex, "default_timezone": original_tz, "maintenance_mode": on }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
            let body: Value = resp.json().await.unwrap();
            assert_eq!(body["maintenance_mode"], on);
        }
    };

    set_maintenance(true).await;

    // A general user's write is rejected...
    let resp = user
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({ "project_id": project_id, "parent_ticket_id": null, "assignee_id": null, "title": "blocked ticket" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503, "general user writes must be rejected during maintenance mode");

    // ...but a read is not.
    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "reads must still work during maintenance mode");

    // An admin's write to an ordinary endpoint is rejected too - maintenance
    // mode must pause everyone's writes, not just non-admins' (a prior bug
    // exempted admins from the gate entirely, which let tickets/work sessions
    // keep being created/started right through an active maintenance window).
    let resp = admin
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({ "project_id": project_id, "parent_ticket_id": null, "assignee_id": null, "title": "blocked admin ticket" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503, "admin writes to ordinary endpoints must be rejected during maintenance mode too");

    // Turning it back off is itself an admin write made *while* maintenance
    // mode is still on - it succeeding is exactly what proves the settings
    // endpoint specifically is exempt (otherwise nobody could ever turn
    // maintenance mode off again).
    set_maintenance(false).await;

    let resp = user
        .client
        .post(format!("{}/api/v1/tickets", base_url()))
        .json(&json!({ "project_id": project_id, "parent_ticket_id": null, "assignee_id": null, "title": "allowed ticket" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "general user writes must succeed again once maintenance mode is off");
}
