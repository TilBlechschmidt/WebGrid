use super::{database, scan};
use crate::libraries::helpers::keys;
use chrono::{DateTime, Duration, TimeZone, Utc};
use log::{debug, error, warn};
use redis::{aio::ConnectionLike, AsyncCommands};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, Executor, Sqlite, SqlitePool};
use std::{
    fs,
    io::{Error as IOError, ErrorKind},
    path::{Path, PathBuf, StripPrefixError},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: f64,
    pub last_modified: DateTime<Utc>,
    pub last_access: DateTime<Utc>,
}

impl FileMetadata {
    pub fn from_fs_metadata(path: PathBuf, metadata: Result<fs::Metadata, std::io::Error>) -> Self {
        let mut size: f64 = 0.0;
        let mut last_modified = Utc::now();
        let mut last_access = Utc::now();

        // Consider dates that are before 2000 or more than 24 hours in the future to be invalid.
        let past_sanity_date = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
        let future_sanity_date = Utc::now() + Duration::hours(24);

        if let Ok(meta) = metadata {
            size = meta.len() as f64;

            if let Ok(modified) = meta.modified() {
                let date_time = modified.into();
                if date_time > past_sanity_date && date_time < future_sanity_date {
                    last_modified = date_time;
                }
            }

            if let Ok(accessed) = meta.accessed() {
                let date_time = accessed.into();
                if date_time > past_sanity_date && date_time < future_sanity_date {
                    last_access = date_time;
                }
            }
        }

        Self {
            path,
            size,
            last_modified,
            last_access,
        }
    }

    pub fn new(path: PathBuf) -> Self {
        let metadata = fs::metadata(&path);

        FileMetadata::from_fs_metadata(path, metadata)
    }
}

/// Filesystem accessor
#[derive(Clone)]
pub struct StorageHandler {
    /// Root directory to watch
    ///
    /// Database will be stored in here
    directory: PathBuf,

    pool: SqlitePool,

    /// Directory size limit in bytes above which a call to `.maybe_cleanup()` triggers
    size_threshold: f64,

    /// Target amount of bytes to delete during cleanup
    cleanup_target: f64,
}

/// Errors thrown during filesystem access
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Unable to access storage database")]
    DatabaseInaccessible(#[from] sqlx::Error),
    #[error("Unknown error")]
    InternalError,
    #[error("File is not in watch directory")]
    StripPrefixError(#[from] StripPrefixError),
    #[error("Failed to serialize metadata")]
    SerializationError(#[from] serde_json::Error),
    #[error("Unable to enqueue metadata")]
    RedisError(#[from] redis::RedisError),
    #[error("Error reading from disk")]
    IOError(#[from] IOError),
}

impl StorageHandler {
    /// Fetches the storage ID of the given directory. If the directory has not previously been used
    /// as a storage a new identifier will be created and written to disk.
    pub async fn storage_id(directory: &PathBuf) -> Result<String, StorageError> {
        let id_path = directory.join(".webgrid-storage");
        debug!("Attempting to read storage ID from {}", id_path.display());

        // Attempt to read existing identifier
        match fs::read_to_string(&id_path) {
            Ok(identifier) => Ok(identifier),

            // Match the kind of error
            Err(e) => match e.kind() {
                // If the file does not exist, create a new one
                ErrorKind::NotFound => {
                    debug!("No storage identifier file found, generating new one.");
                    let identifier = Uuid::new_v4().to_string();
                    fs::write(id_path, &identifier)?;

                    Ok(identifier)
                }

                // If we get any other error (e.g. PermissionDenied) bail
                _ => {
                    warn!("Unable to access storage identifier file: {:?}", e);
                    Err(StorageError::IOError(e))
                }
            },
        }
    }

    pub async fn queue_file_metadata<C: ConnectionLike + AsyncCommands>(
        path: &PathBuf,
        storage_id: &str,
        redis: &mut C,
    ) -> Result<(), StorageError> {
        let metadata = FileMetadata::new(path.to_owned());
        let serialized = serde_json::to_string(&metadata)?;

        redis
            .rpush(keys::storage::metadata::pending(storage_id), serialized)
            .await?;

        Ok(())
    }

    /// Creates new instance that watches a given directory
    /// and attempts to clean at least `cleanup_target` bytes
    /// if the watched directory grows larger than `size_threshold` bytes
    pub async fn new(
        directory: PathBuf,
        size_threshold: f64,
        cleanup_target: f64,
    ) -> Result<Self, StorageError> {
        let database_path = directory.join("storage.db");
        Self::create_db_if_not_exists(&database_path).await?;
        let pool = SqlitePool::connect(&format!("sqlite://{}", database_path.display())).await?;
        let mut con = pool.acquire().await?;
        database::setup_tables(&mut con).await?;

        Ok(Self {
            directory,
            pool,
            size_threshold,
            cleanup_target,
        })
    }

    async fn create_db_if_not_exists(path: &PathBuf) -> Result<(), std::io::Error> {
        if !path.exists() {
            tokio::fs::File::create(path).await?;
        }

        Ok(())
    }

    /// Explicitly scan the full filesystem and sync the database
    ///
    /// This should only be called infrequently as it usually is an expensive operation.
    pub async fn scan_fs(&self) -> Result<(), StorageError> {
        let root = self.directory.clone();
        let transaction = self.pool.begin().await?;
        let scanner = scan::FileSystemScanner::new(transaction, root);

        let resulting_transaction = scanner.scan().await.ok_or(StorageError::InternalError)?;

        resulting_transaction.commit().await?;

        Ok(())
    }

    /// Import or update a file to the database
    ///
    /// Reads the files metadata from disk and upserts the value into the database.
    pub async fn add_file(&self, mut metadata: FileMetadata) -> Result<(), StorageError> {
        let relative_path = self.relative_path(metadata.path)?;
        metadata.path = relative_path;

        let mut con = self.acquire_connection().await?;

        database::insert_file(metadata, &mut con).await?;

        Ok(())
    }

    /// Remove a file from the database
    ///
    /// Call this if you externally delete a file from the watched directory
    #[allow(dead_code)]
    pub async fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<(), StorageError> {
        let mut con = self.acquire_connection().await?;
        let res = self.remove_file_from_con(path, &mut con).await;
        res
    }

    pub async fn used_bytes(&self) -> Result<f64, StorageError> {
        let mut con = self.acquire_connection().await?;
        Ok(database::used_bytes(&mut con).await?)
    }

    /// Runs a cleanup if the used bytes exceed the `size_threshold`
    pub async fn maybe_cleanup(&self) -> Result<usize, StorageError> {
        let mut con = self.acquire_connection().await?;
        let used_bytes: f64 = database::used_bytes(&mut con).await?;

        debug!("Used bytes: {}", used_bytes);

        if used_bytes < self.size_threshold {
            // We are below the threshold, nothing to do here!
            Ok(0)
        } else {
            debug!("Above cleanup threshold!");
            self.cleanup(used_bytes).await
        }
    }

    async fn cleanup(&self, used_bytes: f64) -> Result<usize, StorageError> {
        let mut con = self.acquire_connection().await?;

        // TODO Pass this as a parameter
        database::setup_views(&mut con, "(ModificationTime / 60 / 60 / 24) + (LastAccessTime / 60 / 60 / 24) + (Size / 1024.0 / 1024.0)").await?;

        // Calculate how many bytes are over the limit and include those in the cleanup target size
        let bytes_over_limit = used_bytes - self.size_threshold;
        let cleanup_target = bytes_over_limit + self.cleanup_target;

        let paths = database::files_to_delete(&mut con, cleanup_target).await?;
        let file_count = paths.len();

        for path in paths.into_iter() {
            let absolute_path = self.directory.join(&path);

            if let Err(e) = fs::remove_file(&absolute_path) {
                // TODO Mark this in the database and take it into consideration for future attempts/calculations (maybe ignore its size during accumulation?)
                warn!("Unable to delete file {}: {}", absolute_path.display(), e);
            } else {
                self.remove_file_from_con(&absolute_path, &mut con).await?;
            }
        }

        Ok(file_count)
    }

    async fn remove_file_from_con<'e, P: AsRef<Path>, E>(
        &self,
        path: P,
        con: E,
    ) -> Result<(), StorageError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let relative_path = self.relative_path(path)?;
        let path_str = relative_path.to_str().unwrap_or_default();

        database::delete_file(con, path_str).await?;

        Ok(())
    }

    fn relative_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, StorageError> {
        Ok(path
            .as_ref()
            .strip_prefix(self.directory.as_path())?
            .to_path_buf())
    }

    async fn acquire_connection(&self) -> Result<PoolConnection<Sqlite>, StorageError> {
        let mut con = self.pool.acquire().await?;
        database::setup_tables(&mut con).await?;
        Ok(con)
    }
}
