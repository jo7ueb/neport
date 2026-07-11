use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tar::{Archive, Builder};
use tokio::process::Command;

use crate::error::{AppError, Result};

fn subprocess_err(context: &str, e: impl std::fmt::Debug) -> AppError {
    tracing::error!("{context}: {:?}", e);
    AppError::InternalServerError
}

// A self-deleting scratch directory under the OS temp dir (tmpfs on Cloud
// Run, disk on docker-compose - either way it's local to this process).
pub struct TempDir(PathBuf);

impl TempDir {
    pub async fn new() -> Result<Self> {
        let dir = std::env::temp_dir().join(format!("neport-backup-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| subprocess_err("failed to create temp dir", e))?;
        Ok(TempDir(dir))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// Only the pieces pg_dump/pg_restore need on the command line. Password is
// passed via the PGPASSWORD env var rather than embedded in a connection
// string argument, so it doesn't show up in `ps aux`/argv listings.
struct PgConnInfo {
    host: String,
    port: String,
    user: String,
    password: String,
    dbname: String,
}

fn parse_database_url(database_url: &str) -> Result<PgConnInfo> {
    let url = url::Url::parse(database_url).map_err(|e| subprocess_err("failed to parse DATABASE_URL", e))?;
    Ok(PgConnInfo {
        host: url.host_str().unwrap_or("localhost").to_string(),
        port: url.port().unwrap_or(5432).to_string(),
        user: url.username().to_string(),
        password: url.password().unwrap_or("").to_string(),
        dbname: url.path().trim_start_matches('/').to_string(),
    })
}

pub async fn run_pg_dump(database_url: &str, out_path: &Path) -> Result<()> {
    let info = parse_database_url(database_url)?;
    let output = Command::new("pg_dump")
        .arg("-h").arg(&info.host)
        .arg("-p").arg(&info.port)
        .arg("-U").arg(&info.user)
        .arg("-Fc")
        .arg("-f").arg(out_path)
        .arg(&info.dbname)
        .env("PGPASSWORD", &info.password)
        .stdout(Stdio::null())
        .output()
        .await
        .map_err(|e| subprocess_err("failed to spawn pg_dump", e))?;

    if !output.status.success() {
        tracing::error!("pg_dump exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr));
        return Err(AppError::InternalServerError);
    }
    Ok(())
}

pub async fn run_pg_restore(database_url: &str, dump_path: &Path) -> Result<()> {
    let info = parse_database_url(database_url)?;
    let output = Command::new("pg_restore")
        .arg("--clean").arg("--if-exists").arg("--no-owner")
        .arg("-h").arg(&info.host)
        .arg("-p").arg(&info.port)
        .arg("-U").arg(&info.user)
        .arg("-d").arg(&info.dbname)
        .arg(dump_path)
        .env("PGPASSWORD", &info.password)
        .output()
        .await
        .map_err(|e| subprocess_err("failed to spawn pg_restore", e))?;

    if !output.status.success() {
        tracing::error!("pg_restore exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr));
        return Err(AppError::InternalServerError);
    }
    Ok(())
}

// Synchronous (tar/flate2 are blocking APIs) - callers should run this
// inside tokio::task::spawn_blocking. `uploads` is every attachment's
// (name, bytes) pair, already read from whichever AttachmentStorage backend
// is active (local disk or GCS) - this function itself never touches
// storage directly, so it works the same regardless of backend.
pub fn build_backup_archive(dump_path: &Path, uploads: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let encoder = GzEncoder::new(&mut buf, Compression::default());
        let mut builder = Builder::new(encoder);
        builder
            .append_path_with_name(dump_path, "db.dump")
            .map_err(|e| subprocess_err("failed to add db.dump to backup archive", e))?;
        for (name, data) in uploads {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("uploads/{name}"), data.as_slice())
                .map_err(|e| subprocess_err("failed to add an upload to backup archive", e))?;
        }
        let encoder = builder
            .into_inner()
            .map_err(|e| subprocess_err("failed to finish backup archive", e))?;
        encoder
            .finish()
            .map_err(|e| subprocess_err("failed to finish backup archive compression", e))?;
    }
    Ok(buf)
}

// Synchronous, see build_backup_archive. Extracts db.dump to `dump_out_path`
// and returns every uploads/ entry as an in-memory (name, bytes) pair - the
// caller is responsible for clearing and repopulating whichever
// AttachmentStorage backend is active, since that may require network I/O
// (GCS) that doesn't belong in this blocking, storage-agnostic function.
pub fn extract_backup_archive(data: &[u8], dump_out_path: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    use std::io::Read;

    let decoder = GzDecoder::new(data);
    let mut archive = Archive::new(decoder);
    let mut uploads = Vec::new();
    for entry in archive.entries().map_err(|e| subprocess_err("failed to read backup archive", e))? {
        let mut entry = entry.map_err(|e| subprocess_err("failed to read backup archive entry", e))?;
        let path = entry
            .path()
            .map_err(|e| subprocess_err("failed to read backup archive entry path", e))?
            .into_owned();

        if path == Path::new("db.dump") {
            entry.unpack(dump_out_path).map_err(|e| subprocess_err("failed to extract db.dump", e))?;
            continue;
        }

        if let Ok(rel) = path.strip_prefix("uploads") {
            if rel.as_os_str().is_empty() {
                continue;
            }
            // Defense in depth: tar-rs already rejects absolute/`..` entries
            // in modern versions, but this is an admin-uploaded file we
            // don't otherwise validate, so check explicitly too.
            if rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                continue;
            }
            let Some(name) = rel.to_str() else { continue };
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| subprocess_err("failed to extract attachment file", e))?;
            uploads.push((name.to_string(), buf));
        }
    }
    Ok(uploads)
}
