//! Trait implementations for [Amazon S3](https://aws.amazon.com/s3/) compatible storage providers

use super::StorageBackend;
use async_trait::async_trait;
use http::header::CONTENT_TYPE;
use http::{HeaderMap, StatusCode};
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::error::Error as StdError;
use thiserror::Error;

/// Error type for S3 specific errors
#[derive(Error, Debug)]
pub enum S3StorageError {
    /// Requested object can not be retrieved
    ///
    /// The HTTP status code and error message are provided.
    #[error("object unavailable (status code {0}): {1}")]
    ObjectUnavailable(u16, String),
    /// Internal black-box error caused by the S3 implementation
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Storage backend for AWS S3 compatible servers
#[derive(Clone)]
pub struct S3StorageBackend {
    bucket: Bucket,
}

impl S3StorageBackend {
    /// Creates a new S3 backend connection to a pre-existing bucket using the provided credentials
    pub fn new(
        region: Region,
        credentials: Credentials,
        bucket: &str,
        path_style: bool,
    ) -> Result<Self, S3StorageError> {
        let mut bucket = Bucket::new(bucket, region, credentials)?;

        if path_style {
            bucket.set_path_style();
        }

        Ok(Self { bucket })
    }

    fn handle_response(&self, response: (Vec<u8>, u16)) -> Result<Vec<u8>, S3StorageError> {
        let (data, code) = response;

        if code != StatusCode::OK.as_u16() {
            let reason = String::from_utf8_lossy(&data).to_string();
            Err(S3StorageError::ObjectUnavailable(code, reason))
        } else {
            Ok(data)
        }
    }
}

#[async_trait]
impl StorageBackend for S3StorageBackend {
    fn presign_get(
        &self,
        path: &str,
        expiry_secs: u32,
    ) -> Result<String, Box<dyn StdError + Send + Sync>> {
        Ok(self.bucket.presign_get(path, expiry_secs)?)
    }

    fn presign_put(
        &self,
        path: &str,
        expiry_secs: u32,
        content_type: &str,
    ) -> Result<String, Box<dyn StdError + Send + Sync>> {
        let mut custom_headers = HeaderMap::new();
        custom_headers.insert(CONTENT_TYPE, content_type.parse()?);

        Ok(self.bucket.presign_put(path, expiry_secs, None)?)
    }

    async fn get_object(&self, path: &str) -> Result<Vec<u8>, Box<dyn StdError + Send + Sync>> {
        let response = self.bucket.get_object(path).await?;
        Ok(self.handle_response(response)?)
    }

    async fn put_object(
        &self,
        path: &str,
        content: &[u8],
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let response = self.bucket.put_object(path, content).await?;
        Ok(self.handle_response(response).map(|_| ())?)
    }
}

/// Tests for the S3 storage backend.
/// Note that these are ignored by default as they require a valid S3 endpoint running.
/// The simplest way is to fire up a local minio instance with the details listed in the constants.
///
/// ```bash
/// minio server /tmp/webgrid-minio
/// mc alias set webgrid-test http://127.0.0.1:9000 minioadmin minioadmin
/// mc mb webgrid-test/rust-webgrid-test
/// ```
#[cfg(test)]
mod does {
    use super::*;
    use uuid::Uuid;

    const S3_TEST_BACKEND: &str = "http://127.0.0.1:9000";
    const S3_TEST_BUCKET: &str = "rust-webgrid-test";
    const S3_TEST_ACCESS_KEY: &str = "minioadmin";
    const S3_TEST_SECRET_KEY: &str = "minioadmin";
    const TEST_FILE_CONTENT: &[u8] = &[1, 2, 3, 4];

    // TODO Ensure that the given bucket will be created.

    fn test_backend() -> S3StorageBackend {
        let region = Region::Custom {
            region: S3_TEST_BUCKET.to_string(),
            endpoint: S3_TEST_BACKEND.to_string(),
        };

        let credentials = Credentials::new(
            Some(S3_TEST_ACCESS_KEY),
            Some(S3_TEST_SECRET_KEY),
            None,
            None,
            None,
        )
        .unwrap();

        S3StorageBackend::new(region, credentials, S3_TEST_BUCKET, true).unwrap()
    }

    #[ignore]
    #[tokio::test]
    async fn provide_file_access() {
        let backend = test_backend();
        let path = backend.upload_test_file().await;

        assert_eq!(TEST_FILE_CONTENT, backend.get_object(&path).await.unwrap());

        backend.delete_test_file(path).await;
    }

    #[ignore]
    #[should_panic]
    #[tokio::test]
    async fn fail_on_file_not_found() {
        let backend = test_backend();
        let path = backend.upload_test_file().await;
        backend.delete_test_file(&path).await;
        backend.get_object(&path).await.unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn provide_valid_put_link() {
        let backend = test_backend();
        let path = Uuid::new_v4().to_string();
        let put_link = backend
            .presign_put(&path, 10, &"application/octet-stream".to_string())
            .unwrap();

        let client = reqwest::Client::new();
        let res = client
            .put(put_link)
            .body(TEST_FILE_CONTENT)
            .send()
            .await
            .unwrap();

        assert!(res.status().is_success());
        assert_eq!(TEST_FILE_CONTENT, backend.get_object(&path).await.unwrap());
    }

    #[ignore]
    #[tokio::test]
    async fn provide_valid_get_link() {
        let backend = test_backend();
        let path = backend.upload_test_file().await;

        let get_link = backend.presign_get(&path, 10).unwrap();

        let client = reqwest::Client::new();
        let res = client.get(get_link).send().await.unwrap();

        assert!(res.status().is_success());
        assert_eq!(TEST_FILE_CONTENT, res.bytes().await.unwrap());
    }

    #[async_trait]
    trait TestStorageBackend {
        async fn upload_test_file(&self) -> String;
        async fn delete_test_file<S: AsRef<str> + Send + Sync>(&self, path: S);
    }

    #[async_trait]
    impl TestStorageBackend for S3StorageBackend {
        async fn upload_test_file(&self) -> String {
            let identifier = Uuid::new_v4().to_string();

            self.put_object(&identifier, TEST_FILE_CONTENT)
                .await
                .unwrap();

            identifier
        }

        async fn delete_test_file<S: AsRef<str> + Send + Sync>(&self, path: S) {
            self.bucket.delete_object(path).await.unwrap();
        }
    }
}
