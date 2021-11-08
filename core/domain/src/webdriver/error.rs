use serde::{Deserialize, Serialize};

use library::communication::BlackboxError;

/// Standardised webdriver error codes
pub enum WebdriverErrorCode {
    /// A new session could not be created.
    SessionNotCreated,
    /// An unknown error occurred in the remote end while processing the command.
    UnknownError,
}

impl ToString for WebdriverErrorCode {
    fn to_string(&self) -> String {
        match self {
            WebdriverErrorCode::SessionNotCreated => "session not created".into(),
            WebdriverErrorCode::UnknownError => "unknown error".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct InnerWebdriverError {
    error: String,
    message: String,
    stacktrace: String,
}

/// Specification compliant error response
#[derive(Serialize, Deserialize, Debug)]
pub struct WebdriverError {
    value: InnerWebdriverError,
}

impl WebdriverError {
    /// Creates a new, serializable, spec-compliant error
    pub fn new(error: WebdriverErrorCode, message: String, stacktrace: String) -> Self {
        Self {
            value: InnerWebdriverError {
                error: error.to_string(),
                message,
                stacktrace,
            },
        }
    }
}

impl From<(WebdriverErrorCode, BlackboxError)> for WebdriverError {
    fn from((error, blackbox): (WebdriverErrorCode, BlackboxError)) -> Self {
        let causes = blackbox.into_causes();
        let message = causes
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown error".into());
        let stacktrace = causes.join("\n");

        Self {
            value: InnerWebdriverError {
                error: error.to_string(),
                message,
                stacktrace,
            },
        }
    }
}
