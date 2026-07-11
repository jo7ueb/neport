mod gcs;
mod local;

use std::sync::Arc;

use crate::config::Config;
use crate::error::Result;

// Abstracts "where do attachment bytes live" so backend/src/attachments and
// backend/src/backup don't need to know whether they're talking to the
// docker-compose uploads_data volume or a GCS bucket (Cloud Run has no local
// disk shared across instances/restarts, so attachments there must live in
// GCS instead). `name` is always a caller-validated identifier (sha256 hex
// digest for attachments, or a tar-validated relative path for backup
// restore) - this trait trusts it rather than re-validating.
#[async_trait::async_trait]
pub trait AttachmentStorage: Send + Sync {
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn write(&self, name: &str, data: &[u8]) -> Result<()>;
    // None means "not found", distinct from an actual I/O/network error.
    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>>;
    // Every stored name, for backup creation.
    async fn list(&self) -> Result<Vec<String>>;
    // Deletes everything, for backup restore (which replaces the entire set).
    async fn clear(&self) -> Result<()>;
}

pub async fn build(config: &Config) -> anyhow::Result<Arc<dyn AttachmentStorage>> {
    match &config.gcs_bucket {
        Some(bucket) => Ok(Arc::new(gcs::GcsStorage::new(bucket.clone()).await?)),
        None => Ok(Arc::new(local::LocalDiskStorage::new(&config.upload_dir).await?)),
    }
}
