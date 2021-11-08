//! Structures to persist and retrieve arbitrary binary data

use self::s3::{S3StorageBackend, S3StorageURL};
use super::BoxedError;
use async_trait::async_trait;
use hyper::http::Uri;

pub mod s3;

/// URL from which a storage backend can be instantiated
pub trait StorageURL {
    /// Scheme prefix by which the storage backend can be identified (e.g. with `s3+http://` it would be `s3`)
    fn prefix() -> &'static str;

    /// Creates a new instance by parsing a given [`Uri`]
    fn parse(uri: Uri) -> Option<Self>
    where
        Self: Sized;
}

/// Generic storage backend providing read and write access
#[async_trait]
pub trait StorageBackend: Clone {
    /// URL type from which this backend can be instantiated
    type URL: StorageURL;

    /// Instantiates a new backend from the parsed URL
    fn new(url: Self::URL) -> Result<Self, BoxedError>;

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

/// Parses a storage backend URI and instantiates the matching backend
pub fn parse_storage_backend_uri(input: &str) -> Result<S3StorageBackend, BoxedError> {
    let (prefix, raw_uri) = input.split_once('+').unwrap();
    let uri: Uri = raw_uri.parse()?;

    // Currently, there is only an S3 storage backend.
    if prefix != "s3" {
        return Err(anyhow::anyhow!("Storage backend URI should be prefixed with s3+").into());
    }

    let url =
        S3StorageURL::parse(uri).ok_or_else(|| anyhow::anyhow!("Unable to parse storage URI"))?;

    S3StorageBackend::new(url)
}
