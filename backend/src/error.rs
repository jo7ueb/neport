use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("CSRF token missing or invalid")]
    InvalidCsrf,

    #[error("Not found")]
    NotFound,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("System is under maintenance")]
    MaintenanceMode,

    #[error("Internal server error")]
    InternalServerError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // error_id is only generated for the two "this should not happen"
        // variants - the ones where the message shown to the user can't
        // explain the actual cause, so the only way to debug a user's
        // report is to grep the backend log for this id and read the
        // detailed tracing::error! line logged right next to it. Expected
        // errors (bad request, not found, forbidden, ...) already carry a
        // self-explanatory message and don't need one.
        let (status, error_message, error_id) = match self {
            AppError::Database(e) => {
                let error_id = Uuid::new_v4();
                tracing::error!(%error_id, "Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                    Some(error_id),
                )
            }
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string(), None),
            AppError::InvalidCsrf => (StatusCode::FORBIDDEN, self.to_string(), None),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), None),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, None),
            AppError::MaintenanceMode => (StatusCode::SERVICE_UNAVAILABLE, self.to_string(), None),
            AppError::InternalServerError => {
                let error_id = Uuid::new_v4();
                tracing::error!(%error_id, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                    Some(error_id),
                )
            }
        };

        let mut body = json!({ "error": error_message });
        if let Some(error_id) = error_id {
            body["error_id"] = json!(error_id.to_string());
        }

        (status, Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

// Make AppError work with ? operator in handlers
impl<T> From<AppError> for std::result::Result<T, AppError> {
    fn from(err: AppError) -> Self {
        Err(err)
    }
}
