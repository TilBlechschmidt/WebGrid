use log::{debug, error, warn};
use sqlx::{pool::PoolConnection, Connection, Executor, Sqlite, SqliteConnection, SqlitePool};
use std::{
    fs,
    io::{Error as IOError, ErrorKind},
    path::{Path, PathBuf, StripPrefixError},
};
use thiserror::Error;
use uuid::Uuid;

use super::{database, scan};

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
    #[error("Error reading from disk")]
    IOError(#[from] IOError),
}

impl StorageHandler {
    /// Fetches the storage ID of the given directory. If the directory has not previously been used
    /// as a storage a new identifier will be created and written to disk.
    pub async fn storage_id(directory: PathBuf) -> Result<String, StorageError> {
        let id_path = directory.join(".webgrid-storage");

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
                    warn!("Unable to access storage identifier file.");
                    Err(StorageError::IOError(e))
                }
            },
        }
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
        let pool = SqlitePool::new(&format!("sqlite://{}", database_path.display())).await?;
        let mut con = pool.acquire().await?;
        database::setup_tables(&mut con).await?;

        Ok(Self {
            directory,
            pool,
            size_threshold,
            cleanup_target,
        })
    }

    /// Explicitly scan the full filesystem and sync the database
    ///
    /// This should only be called infrequently as it usually is an expensive operation.
    pub async fn scan_fs(&self) -> Result<(), StorageError> {
        let root = self.directory.clone();
        let transaction = self.pool.begin().await?;

        let scanner = scan::FileSystemScanner::new(transaction, root);

        let resulting_transaction = scanner
            .scan()
            .await
            .ok_or_else(|| StorageError::InternalError)?;

        let con = resulting_transaction.commit().await?;
        con.close();

        Ok(())
    }

    /// Import or update a file to the database
    ///
    /// Reads the files metadata from disk and upserts the value into the database.
    pub async fn add_file<P: AsRef<Path>>(&self, path: P) -> Result<(), StorageError> {
        let metadata = fs::metadata(&path);

        let relative_path = self.relative_path(path)?;
        let path_str = relative_path.to_str().unwrap_or_default();

        let mut con = self.acquire_connection().await?;

        database::insert_file(path_str, metadata.ok(), &mut con).await?;
        con.close();

        Ok(())
    }

    /// Remove a file from the database
    ///
    /// Call this if you externally delete a file from the watched directory
    #[allow(dead_code)]
    pub async fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<(), StorageError> {
        let mut con = self.acquire_connection().await?;
        let res = self.remove_file_from_con(path, &mut con).await;
        con.close();
        res
    }

    /// Runs a cleanup if the used bytes exceed the `size_threshold`
    pub async fn maybe_cleanup(&self) -> Result<usize, StorageError> {
        let mut con = self.acquire_connection().await?;
        let used_bytes: f64 = database::used_bytes(&mut con).await?;
        con.close();

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

        con.close();

        Ok(file_count)
    }

    async fn remove_file_from_con<P: AsRef<Path>, E: Executor<Database = Sqlite>>(
        &self,
        path: P,
        mut con: E,
    ) -> Result<(), StorageError> {
        let relative_path = self.relative_path(path)?;
        let path_str = relative_path.to_str().unwrap_or_default();

        database::delete_file(&mut con, path_str).await?;

        Ok(())
    }

    fn relative_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, StorageError> {
        Ok(path
            .as_ref()
            .strip_prefix(self.directory.as_path())?
            .to_path_buf())
    }

    async fn acquire_connection(&self) -> Result<PoolConnection<SqliteConnection>, StorageError> {
        let mut con = self.pool.acquire().await?;
        database::setup_tables(&mut con).await?;
        Ok(con)
    }
}
