use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Request data

/// Incoming request to create a session
#[derive(Deserialize, Debug)]
pub struct SessionCreationRequest {
    /// Requested capabilities. These are not parsed further as they may contain extension fields
    /// which should be passed down to other components.
    pub capabilities: Value,
}

// --- Response data

/// Inner response to a [`SessionCreationRequest`]
#[derive(Serialize, Deserialize)]
pub struct SessionCreateResponseValue {
    /// Identifier of the created session
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Resulting capabilities
    pub capabilities: Value,
}

/// Shell-response to a [`SessionCreationRequest`]
#[derive(Serialize, Deserialize)]
pub struct SessionCreateResponse {
    /// Wrapped response
    pub value: SessionCreateResponseValue,
}
