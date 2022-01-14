use async_trait::async_trait;
use async_zip::error::ZipError;
use async_zip::read::mem::ZipFileReader;
use futures::Future;
use harness::HeartStone;
use hyper::header::CONTENT_TYPE;
use hyper::{
    http::{request::Parts, Method, Response},
    Body,
};
use library::http::Responder;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::net::IpAddr;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::copy;
use tokio::sync::Mutex;
use tracing::debug;

#[derive(Deserialize)]
struct FileUploadRequest {
    file: String,
}

#[derive(Debug, Error)]
enum FileUploadInterceptorError {
    #[error("failed unpacking archive {0:?}")]
    InvalidArchive(ZipError),
    #[error("incomplete request body")]
    UploadError(#[from] hyper::Error),
    #[error("invalid request format")]
    RequestFormatInvalid(#[from] serde_json::Error),
    #[error("invalid base64 encoded file string")]
    FileStringInvalid(#[from] base64::DecodeError),
}

pub struct FileUploadInterceptor {
    heart_stone: Arc<Mutex<HeartStone>>,
    session_id: String,
    file_handles: Arc<Mutex<Vec<TempDir>>>,
}

impl FileUploadInterceptor {
    pub fn new(heart_stone: HeartStone, session_id: String) -> Self {
        Self {
            heart_stone: Arc::new(Mutex::new(heart_stone)),
            session_id,
            file_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn handle_body(&self, body: Body) -> Result<String, FileUploadInterceptorError> {
        let request_bytes = hyper::body::to_bytes(body).await?;
        let request: FileUploadRequest = serde_json::from_slice(&request_bytes)?;
        let bytes = base64::decode(&request.file)?;

        // Get a handle on the first file in the ZIP
        let mut zip_reader = ZipFileReader::new(&bytes)
            .await
            .map_err(FileUploadInterceptorError::InvalidArchive)?;
        let mut file_reader = zip_reader
            .entry_reader(0)
            .await
            .map_err(FileUploadInterceptorError::InvalidArchive)?;

        // Create a file on disk
        let name = file_reader.entry().name();
        let directory = tempdir().unwrap();
        let file_path = directory.path().join(name);
        let mut file_writer = File::create(&file_path).await.unwrap();

        // Write the contents and retain the handle
        copy(&mut file_reader, &mut file_writer).await.unwrap();
        self.file_handles.lock().await.push(directory);

        // Send the path to the caller
        debug!(?file_path, "Received file upload from client");
        Ok(file_path.display().to_string())
    }
}

#[async_trait]
impl Responder for FileUploadInterceptor {
    #[inline]
    async fn respond<F, Fut>(
        &self,
        parts: Parts,
        body: Body,
        client_ip: IpAddr,
        next: F,
    ) -> Result<Response<Body>, Infallible>
    where
        Fut: Future<Output = Result<Response<Body>, Infallible>> + Send,
        F: FnOnce(Parts, Body, IpAddr) -> Fut + Send,
    {
        // Reset the lifetime
        self.heart_stone.lock().await.reset_lifetime().await;

        // Check if it is a file upload request
        let method = &parts.method;
        let path = parts.uri.path();

        let is_file_upload_request = method == Method::POST
            && path.eq_ignore_ascii_case(&format!("/session/{}/se/file", self.session_id));

        // Handle the file and json-ify the result
        if is_file_upload_request {
            let response_value = match self.handle_body(body).await {
                Ok(path) => json!({ "value": path }),
                Err(e) => json!({
                    "status": "error",
                    "error": e.to_string()
                }),
            };

            // Build a json response and send it
            let response = serde_json::to_string(&response_value).unwrap_or_else(|_| "{}".into());
            Ok(Response::builder()
                .header(CONTENT_TYPE, "application/json")
                .body(response.into())
                .unwrap())
        } else {
            next(parts, body, client_ip).await
        }
    }
}
