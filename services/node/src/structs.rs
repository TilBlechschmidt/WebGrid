use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Error as IOError;

#[derive(Debug)]
pub enum NodeError {
    DriverStart(IOError),
    NoDriverResponse,
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
