use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{auth::extractors::AuthUser, error::{AppError, Result}, state::AppState};
use super::repo;

// (mime type, file extension) - the extension is only ever used to build the
// on-disk filename; what actually gets sent back as Content-Type always
// comes from the mime type recorded in the database at upload time.
const ALLOWED_TYPES: &[(&str, &str)] = &[
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
];

fn extension_for(content_type: &str) -> Option<&'static str> {
    ALLOWED_TYPES.iter().find(|(mime, _)| *mime == content_type).map(|(_, ext)| *ext)
}

#[derive(Debug, Serialize)]
pub struct UploadedAttachment {
    pub url: String,
}

pub async fn upload_attachment(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    mut multipart: axum::extract::Multipart,
) -> Result<(StatusCode, Json<UploadedAttachment>)> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("invalid upload".into()))?
    {
        if field.name() == Some("file") {
            content_type = field.content_type().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("failed to read uploaded file".into()))?;
            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| AppError::BadRequest("no file uploaded".into()))?;
    let content_type = content_type.ok_or_else(|| AppError::BadRequest("missing file type".into()))?;
    let extension = extension_for(&content_type)
        .ok_or_else(|| AppError::BadRequest("unsupported image type (must be PNG, JPEG, WebP, or GIF)".into()))?;

    if data.len() > super::MAX_ATTACHMENT_BYTES {
        return Err(AppError::BadRequest("image too large (max 8MB)".into()));
    }

    let sha256 = hex::encode(Sha256::digest(&data));
    let filename = format!("{sha256}.{extension}");

    // Content-addressed storage: if a file with this hash already exists,
    // its bytes are guaranteed identical, so there's nothing to write.
    if !state.storage.exists(&filename).await? {
        state.storage.write(&filename, &data).await?;
    }

    repo::upsert(&state.db, &sha256, &content_type, data.len() as i64, Some(user.id)).await?;

    Ok((
        StatusCode::CREATED,
        Json(UploadedAttachment { url: format!("/api/v1/attachments/{filename}") }),
    ))
}

pub async fn get_attachment(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
    Path(requested): Path<String>,
) -> Result<impl IntoResponse> {
    // sha256 hashes are exactly 64 lowercase hex characters - reject
    // anything else before it ever reaches a filesystem path, rather than
    // trusting the client-supplied URL segment.
    let sha256 = requested.split('.').next().unwrap_or("");
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError::NotFound);
    }

    let content_type = repo::get_content_type(&state.db, sha256).await?.ok_or(AppError::NotFound)?;
    let extension = extension_for(&content_type).ok_or(AppError::InternalServerError)?;
    let data = state
        .storage
        .read(&format!("{sha256}.{extension}"))
        .await?
        .ok_or(AppError::NotFound)?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "private, max-age=31536000, immutable".to_string()),
        ],
        data,
    ))
}
