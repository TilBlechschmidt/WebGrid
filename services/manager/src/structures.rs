use redis::RedisError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use warp::reject::Reject;

// --- Errors

#[derive(Debug)]
pub enum RequestError {
    RedisError(RedisError),
    QueueTimeout,
    SchedulingTimeout,
    HealthCheckTimeout,
    ParseError,
    NoOrchestratorAvailable,
}

impl Reject for RequestError {}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            RequestError::RedisError(e) => {
                write!(f, "Error while interacting with database: {}", e)
            }
            RequestError::QueueTimeout => {
                write!(f, "Timed out while waiting for a free WebDriver slot")
            }
            RequestError::SchedulingTimeout => {
                write!(f, "Timed out while waiting for orchestrator to respond")
            }
            RequestError::HealthCheckTimeout => write!(
                f,
                "Timed out while waiting for the WebGrid-Node to become responsive"
            ),
            RequestError::ParseError => write!(f, "Failed to parse response from database"),
            RequestError::NoOrchestratorAvailable => write!(
                f,
                "No orchestrator available that can satisfy the required capabilities"
            ),
        }
    }
}

// --- Request data

#[derive(Deserialize, Debug)]
pub struct SessionRequest {
    pub capabilities: Value,
}

// --- Response data

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionReplyValue {
    pub session_id: String,
    pub capabilities: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReplyError {
    pub error: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReply {
    pub value: Value,
}
