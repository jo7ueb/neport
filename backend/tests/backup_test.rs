mod common;
use common::{admin_client, as_i64, base_url, unique, TestUser};
use serde_json::{json, Value};

// Full backup/restore round trip through the real HTTP API. pg_restore
// --clean wipes and recreates the entire test-db, which is disruptive to
// any other test running against it - but cargo runs separate integration
// test binaries one at a time (never overlapping), so keeping every
// destructive step inside this one test function in its own file (same
// approach as settings_test.rs's maintenance-mode test) means nothing else
// can race with it.
#[tokio::test]
async fn backup_and_restore_round_trips_through_the_real_api() {
    let admin = admin_client().await;
    let user = TestUser::create().await;
    let project = user.create_project(&unique("backup-project")).await;
    let project_id = as_i64(&project, "id");
    let project_name = project["name"].as_str().unwrap().to_string();

    // General users must not be able to touch either endpoint.
    let resp = user.client.post(format!("{}/api/v1/backup", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 403);
    let resp = user
        .client
        .post(format!("{}/api/v1/backup/restore", base_url()))
        .multipart(reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(vec![]).file_name("x")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let admin_id = admin
        .get(format!("{}/api/v1/auth/me", base_url()))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["user"]["id"]
        .as_i64()
        .unwrap();

    // Take a real backup as admin.
    let resp = admin.post(format!("{}/api/v1/backup", base_url())).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "application/gzip");
    let bundle = resp.bytes().await.unwrap().to_vec();
    assert!(!bundle.is_empty(), "backup bundle should not be empty");

    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), admin_id))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(
        body["items"].as_array().unwrap().iter().any(|e| e["action"] == "backup_created"),
        "taking a backup must leave an audit_log entry"
    );

    // Restoring without maintenance mode enabled must be rejected.
    let resp = admin
        .post(format!("{}/api/v1/backup/restore", base_url()))
        .multipart(reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(bundle.clone()).file_name("backup.tar.gz"),
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "restoring without maintenance mode enabled must be rejected");

    // Enable maintenance mode (required precondition for restore).
    let settings_resp = admin.get(format!("{}/api/v1/settings", base_url())).send().await.unwrap();
    let original: Value = settings_resp.json().await.unwrap();
    let original_katex = original["katex_enabled"].as_bool().unwrap();
    let original_tz = original["default_timezone"].as_str().unwrap().to_string();

    let resp = admin
        .patch(format!("{}/api/v1/settings", base_url()))
        .json(&json!({ "katex_enabled": original_katex, "default_timezone": original_tz, "maintenance_mode": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Restore the exact bundle we just took.
    let resp = admin
        .post(format!("{}/api/v1/backup/restore", base_url()))
        .multipart(
            reqwest::multipart::Form::new()
                .part("file", reqwest::multipart::Part::bytes(bundle).file_name("backup.tar.gz")),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204, "restore should succeed once maintenance mode is enabled");

    // The restore replaces audit_log's own contents with the dump taken
    // earlier (before backup_created/system_settings_updated existed), so
    // those must be gone now - but backup_restored, inserted right after
    // pg_restore finishes, must still show up.
    let resp = admin
        .get(format!("{}/api/v1/audit-log?user_id={}", base_url(), admin_id))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    let entries = body["items"].as_array().unwrap();
    assert!(
        entries.iter().any(|e| e["action"] == "backup_restored"),
        "restoring a backup must leave an audit_log entry that survives the restore itself"
    );
    assert!(
        !entries.iter().any(|e| e["action"] == "backup_created"),
        "audit_log entries written after the dump was taken must not survive the restore"
    );

    // Turn maintenance mode back off.
    let resp = admin
        .patch(format!("{}/api/v1/settings", base_url()))
        .json(&json!({ "katex_enabled": original_katex, "default_timezone": original_tz, "maintenance_mode": false }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // The general user's original session (and the project it created)
    // must still work after the restore - the session row itself is part
    // of the dump, so this also proves the same cookie still authenticates.
    let resp = user
        .client
        .get(format!("{}/api/v1/projects/{}", base_url(), project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "the same session must still work after restore");
    let restored_project: Value = resp.json().await.unwrap();
    assert_eq!(restored_project["name"], project_name);
}
