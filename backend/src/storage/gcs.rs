use bytes::Bytes;
use google_cloud_gax::paginator::ItemPaginator;
use google_cloud_storage::client::{Storage, StorageControl};

use super::AttachmentStorage;
use crate::error::{AppError, Result};

fn gcs_err(context: &str, e: impl std::fmt::Debug) -> AppError {
    tracing::error!("{context}: {:?}", e);
    AppError::InternalServerError
}

// Cloud Run has no local disk shared across instances/restarts, so
// attachments there live in a GCS bucket instead of backend/src/storage/local.rs's
// uploads_data volume. Auth is Application Default Credentials - on Cloud
// Run this transparently uses the service's attached service account via the
// metadata server; nothing else needs configuring.
pub struct GcsStorage {
    bucket: String,
    storage: Storage,
    control: StorageControl,
}

impl GcsStorage {
    pub async fn new(bucket: String) -> anyhow::Result<Self> {
        let storage = Storage::builder().build().await?;
        let control = StorageControl::builder().build().await?;
        Ok(GcsStorage { bucket, storage, control })
    }

    // Bucket names are passed to this API in `projects/_/buckets/{bucket}`
    // form; `_` stands in for the project since bucket names are already
    // globally unique.
    fn bucket_resource(&self) -> String {
        format!("projects/_/buckets/{}", self.bucket)
    }
}

#[async_trait::async_trait]
impl AttachmentStorage for GcsStorage {
    async fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.read(name).await?.is_some())
    }

    async fn write(&self, name: &str, data: &[u8]) -> Result<()> {
        self.storage
            .write_object(self.bucket_resource(), name, Bytes::from(data.to_vec()))
            .send_buffered()
            .await
            .map_err(|e| gcs_err(&format!("failed to write {name} to GCS"), e))?;
        Ok(())
    }

    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let mut resp = match self.storage.read_object(self.bucket_resource(), name).send().await {
            Ok(resp) => resp,
            Err(e) if e.http_status_code() == Some(404) => return Ok(None),
            Err(e) => return Err(gcs_err(&format!("failed to open {name} for read from GCS"), e)),
        };

        let mut contents = Vec::new();
        loop {
            match resp.next().await.transpose() {
                Ok(Some(chunk)) => contents.extend_from_slice(&chunk),
                Ok(None) => break,
                Err(e) if e.http_status_code() == Some(404) => return Ok(None),
                Err(e) => return Err(gcs_err(&format!("failed to read {name} from GCS"), e)),
            }
        }
        Ok(Some(contents))
    }

    async fn list(&self) -> Result<Vec<String>> {
        let mut items = self
            .control
            .list_objects()
            .set_parent(self.bucket_resource())
            .by_item();

        let mut names = Vec::new();
        while let Some(item) = items.next().await {
            let object = item.map_err(|e| gcs_err("failed to list GCS objects", e))?;
            names.push(object.name);
        }
        Ok(names)
    }

    async fn clear(&self) -> Result<()> {
        for name in self.list().await? {
            self.control
                .delete_object()
                .set_bucket(self.bucket_resource())
                .set_object(name.clone())
                .send()
                .await
                .map_err(|e| gcs_err(&format!("failed to delete {name} from GCS"), e))?;
        }
        Ok(())
    }
}
