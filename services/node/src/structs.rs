use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Error as IOError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("unable to launch driver")]
    DriverStart(IOError),
    #[error("no status response from driver")]
    NoDriverResponse,
    #[error("failed to create local session")]
    LocalSessionCreationError,
}

#[derive(Serialize, Deserialize)]
pub struct SessionCreateResponseValue {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub capabilities: Value,
}

#[derive(Deserialize)]
pub struct SessionCreateResponse {
    pub value: SessionCreateResponseValue,
}
