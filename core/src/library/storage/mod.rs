//! Structures to persist and retrieve arbitrary binary data

use super::BoxedError;
use async_trait::async_trait;

#[cfg(feature = "storage")]
pub mod s3;

/// Generic storage backend providing read and write access
#[async_trait]
pub trait StorageBackend {
    /// Retrieves a pre-signed link which can be used by anyone to read a given resource
    fn presign_get(&self, path: &str, expiry_secs: u32) -> Result<String, BoxedError>;

    /// Retrieves a pre-signed link which can be used by anyone to write a given resource
    fn presign_put(
        &self,
        path: &str,
        expiry_secs: u32,
        content_type: &str,
    ) -> Result<String, BoxedError>;

    /// Retrieves the content of a stored object
    async fn get_object(&self, path: &str) -> Result<Vec<u8>, BoxedError>;

    /// Creates a new object at the given path
    async fn put_object(&self, path: &str, content: &[u8]) -> Result<(), BoxedError>;
}
