use std::path::PathBuf;

use super::AttachmentStorage;
use crate::error::{AppError, Result};

fn io_err(context: &str, e: impl std::fmt::Debug) -> AppError {
    tracing::error!("{context}: {:?}", e);
    AppError::InternalServerError
}

pub struct LocalDiskStorage {
    dir: PathBuf,
}

impl LocalDiskStorage {
    pub async fn new(dir: &str) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(dir).await?;
        Ok(LocalDiskStorage { dir: PathBuf::from(dir) })
    }
}

#[async_trait::async_trait]
impl AttachmentStorage for LocalDiskStorage {
    async fn exists(&self, name: &str) -> Result<bool> {
        Ok(tokio::fs::metadata(self.dir.join(name)).await.is_ok())
    }

    async fn write(&self, name: &str, data: &[u8]) -> Result<()> {
        tokio::fs::write(self.dir.join(name), data)
            .await
            .map_err(|e| io_err(&format!("failed to write {name} to local storage"), e))
    }

    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        match tokio::fs::read(self.dir.join(name)).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_err(&format!("failed to read {name} from local storage"), e)),
        }
    }

    async fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.dir)
            .await
            .map_err(|e| io_err("failed to read local storage directory", e))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| io_err("failed to read local storage directory entry", e))?
        {
            if entry.path().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
        Ok(names)
    }

    async fn clear(&self) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.dir)
            .await
            .map_err(|e| io_err("failed to read local storage directory", e))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| io_err("failed to read local storage directory entry", e))?
        {
            let path = entry.path();
            let result = if path.is_dir() {
                tokio::fs::remove_dir_all(&path).await
            } else {
                tokio::fs::remove_file(&path).await
            };
            result.map_err(|e| io_err("failed to clear a local storage entry", e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn write_exists_read_list_clear_round_trip() {
        let tmp = std::env::temp_dir().join(format!("neport-storage-test-{}", uuid::Uuid::new_v4()));
        let storage = LocalDiskStorage::new(tmp.to_str().unwrap()).await.unwrap();

        assert!(!storage.exists("a.txt").await.unwrap());
        assert_eq!(storage.read("a.txt").await.unwrap(), None);

        storage.write("a.txt", b"hello").await.unwrap();
        assert!(storage.exists("a.txt").await.unwrap());
        assert_eq!(storage.read("a.txt").await.unwrap(), Some(b"hello".to_vec()));
        assert_eq!(storage.list().await.unwrap(), vec!["a.txt".to_string()]);

        storage.clear().await.unwrap();
        assert!(!storage.exists("a.txt").await.unwrap());
        assert_eq!(storage.list().await.unwrap(), Vec::<String>::new());

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }
}
