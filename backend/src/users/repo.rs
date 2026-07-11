use crate::auth::{User, UserRole, UserStatus};
use crate::error::Result;

pub(crate) type UserRow = (i64, String, String, String, String, String, Option<chrono::DateTime<chrono::Utc>>, bool);

pub(crate) fn row_to_user(row: UserRow) -> User {
    let (id, email, display_name, role_str, status_str, locale, avatar_updated_at, is_approver) = row;
    User {
        id,
        email,
        display_name,
        role: role_str.parse().unwrap_or(UserRole::General),
        status: status_str.parse().unwrap_or(UserStatus::Active),
        locale,
        avatar_updated_at,
        is_approver,
    }
}

pub async fn create_user(
    db: &sqlx::PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
    role: UserRole,
    created_by_id: Option<i64>,
) -> Result<User> {
    let role_str = role.to_string();
    let user = sqlx::query_as::<_, (i64, String, String, String, String)>(
        "INSERT INTO users (email, password_hash, display_name, role, created_by_id) VALUES ($1, $2, $3, $4, $5) RETURNING id, email, display_name, role, locale",
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(&role_str)
    .bind(created_by_id)
    .fetch_one(db)
    .await?;

    Ok(User {
        id: user.0,
        email: user.1,
        display_name: user.2,
        role,
        status: UserStatus::Active,
        locale: user.4,
        avatar_updated_at: None,
        is_approver: false,
    })
}

pub async fn count_users(db: &sqlx::PgPool) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await?;
    Ok(count)
}

pub async fn get_user_by_email(db: &sqlx::PgPool, email: &str) -> Result<Option<UserWithPassword>> {
    let row = sqlx::query_as::<_, (i64, String, String, String, String, String, Option<chrono::DateTime<chrono::Utc>>, bool)>(
        "SELECT id, email, password_hash, display_name, role, locale, avatar_updated_at, is_approver FROM users WHERE email = $1 AND status != 'deleted'",
    )
    .bind(email)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(id, email, password_hash, display_name, role_str, locale, avatar_updated_at, is_approver)| {
        let role: UserRole = role_str.parse().unwrap_or(UserRole::General);
        UserWithPassword {
            user: User {
                id,
                email,
                display_name,
                role,
                status: UserStatus::Active,
                locale,
                avatar_updated_at,
                is_approver,
            },
            password_hash,
        }
    }))
}

pub async fn get_user_by_id(db: &sqlx::PgPool, id: i64) -> Result<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, display_name, role, status, locale, avatar_updated_at, is_approver FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(row_to_user))
}

pub async fn list_users(db: &sqlx::PgPool, limit: i64, offset: i64) -> Result<Vec<User>> {
    let rows = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, display_name, role, status, locale, avatar_updated_at, is_approver FROM users WHERE status != 'deleted' ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(row_to_user).collect())
}

// Escapes ILIKE wildcards so search input is matched literally rather than
// as a pattern, mirroring tickets::repo::search_tickets.
pub async fn search_users(db: &sqlx::PgPool, query: &str, limit: i64) -> Result<Vec<User>> {
    let pattern = format!("%{}%", query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let rows = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, display_name, role, status, locale, avatar_updated_at, is_approver FROM users \
         WHERE status != 'deleted' AND ($1 = '' OR display_name ILIKE $2 OR email ILIKE $2) \
         ORDER BY display_name LIMIT $3",
    )
    .bind(query)
    .bind(pattern)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(row_to_user).collect())
}

pub async fn count_listed_users(db: &sqlx::PgPool) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE status != 'deleted'")
        .fetch_one(db)
        .await?;
    Ok(count)
}

pub async fn suspend_user(db: &sqlx::PgPool, user_id: i64) -> Result<()> {
    sqlx::query("UPDATE users SET status = 'suspended', updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn resume_user(db: &sqlx::PgPool, user_id: i64) -> Result<()> {
    sqlx::query("UPDATE users SET status = 'active', updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn update_role(db: &sqlx::PgPool, user_id: i64, role: UserRole) -> Result<()> {
    sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
        .bind(role.to_string())
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn set_is_approver(db: &sqlx::PgPool, user_id: i64, is_approver: bool) -> Result<()> {
    sqlx::query("UPDATE users SET is_approver = $1, updated_at = NOW() WHERE id = $2")
        .bind(is_approver)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

// The effective timezone to bucket this user's timestamps into calendar
// days: their own override if set, otherwise the system-wide default.
pub async fn resolve_timezone(db: &sqlx::PgPool, user_id: i64) -> Result<String> {
    let (timezone,): (String,) = sqlx::query_as(
        "SELECT COALESCE(u.timezone, s.default_timezone) FROM users u, system_settings s WHERE u.id = $1 AND s.id = 1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;
    Ok(timezone)
}

pub async fn get_timezone_override(db: &sqlx::PgPool, user_id: i64) -> Result<Option<String>> {
    let (timezone,): (Option<String>,) = sqlx::query_as("SELECT timezone FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await?;
    Ok(timezone)
}

pub async fn set_timezone(db: &sqlx::PgPool, user_id: i64, timezone: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE users SET timezone = $1, updated_at = NOW() WHERE id = $2")
        .bind(timezone)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn update_locale(db: &sqlx::PgPool, user_id: i64, locale: &str) -> Result<()> {
    sqlx::query("UPDATE users SET locale = $1, updated_at = NOW() WHERE id = $2")
        .bind(locale)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn update_display_name(db: &sqlx::PgPool, user_id: i64, display_name: &str) -> Result<()> {
    sqlx::query("UPDATE users SET display_name = $1, updated_at = NOW() WHERE id = $2")
        .bind(display_name)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn update_avatar(db: &sqlx::PgPool, user_id: i64, data: &[u8], content_type: &str) -> Result<()> {
    sqlx::query(
        "UPDATE users SET avatar_data = $1, avatar_content_type = $2, avatar_updated_at = NOW(), updated_at = NOW() WHERE id = $3",
    )
    .bind(data)
    .bind(content_type)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_avatar(db: &sqlx::PgPool, user_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE users SET avatar_data = NULL, avatar_content_type = NULL, avatar_updated_at = NULL, updated_at = NOW() WHERE id = $1",
    )
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn get_avatar(db: &sqlx::PgPool, user_id: i64) -> Result<Option<(Vec<u8>, String)>> {
    let row: Option<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT avatar_data, avatar_content_type FROM users WHERE id = $1 AND avatar_data IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    Ok(row)
}

pub async fn update_password_hash(db: &sqlx::PgPool, user_id: i64, password_hash: &str) -> Result<()> {
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(password_hash)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_password_hash_by_id(db: &sqlx::PgPool, user_id: i64) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await?;

    Ok(row.map(|r| r.0))
}

pub struct UserWithPassword {
    pub user: User,
    pub password_hash: String,
}
