use anyhow::Result;

pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub session_cookie_name: String,
    pub session_ttl_seconds: u64,
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
    pub admin_display_name: Option<String>,
    pub login_rate_limit_per_second: u64,
    pub login_rate_limit_burst: u32,
    pub upload_dir: String,
    // When set, attachments live in this GCS bucket instead of upload_dir
    // (see backend/src/storage) - used for Cloud Run, where instances have
    // no local disk shared across restarts/instances. docker-compose never
    // sets this, so it always resolves to the local-disk storage backend.
    pub gcs_bucket: Option<String>,
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://neport:neport_dev_password@localhost:5432/neport".to_string()),
            // Cloud Run injects PORT and requires the container to listen on
            // it; docker-compose always sets BIND_ADDR explicitly, so this
            // fallback never changes docker-compose's behavior.
            bind_addr: std::env::var("BIND_ADDR")
                .ok()
                .or_else(|| std::env::var("PORT").ok().map(|p| format!("0.0.0.0:{p}")))
                .unwrap_or_else(|| "0.0.0.0:3000".to_string()),
            session_cookie_name: std::env::var("SESSION_COOKIE_NAME")
                .unwrap_or_else(|_| "neport_session".to_string()),
            session_ttl_seconds: std::env::var("SESSION_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(86400),
            admin_email: non_empty_env("ADMIN_EMAIL"),
            admin_password: non_empty_env("ADMIN_PASSWORD"),
            admin_display_name: non_empty_env("ADMIN_DISPLAY_NAME"),
            // Defaults are tuned for production (behind nginx, one IP per
            // real client). Local/dev/test docker-compose overrides these in
            // .env to a much larger burst, since the integration test suite
            // bypasses nginx and hits this backend directly, so every test's
            // login call shares one apparent source IP.
            login_rate_limit_per_second: std::env::var("LOGIN_RATE_LIMIT_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(12),
            login_rate_limit_burst: std::env::var("LOGIN_RATE_LIMIT_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            upload_dir: std::env::var("UPLOAD_DIR")
                .unwrap_or_else(|_| "/app/uploads".to_string()),
            gcs_bucket: non_empty_env("GCS_BUCKET"),
        })
    }
}
