use std::collections::HashMap;

use super::event::{
    ProvisionedSessionMetadata, ProvisionerIdentifier, SessionIdentifier, SessionTerminationReason,
};
use crate::library::helpers::option_chrono_datetime_as_bson_datetime;
use chrono::{DateTime, Utc};
use mongodb::bson::serde_helpers::uuid_as_binary;
use serde::{Deserialize, Serialize};

/// Indexable metadata for a session
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetadata {
    /// Unique identifier of a session
    #[serde(rename = "_id", with = "uuid_as_binary")]
    pub id: SessionIdentifier,

    /// Time at which the session object was initially created
    #[serde(with = "option_chrono_datetime_as_bson_datetime")]
    pub created_at: Option<DateTime<Utc>>,

    /// Time at which the session was scheduled with a provisioner
    #[serde(with = "option_chrono_datetime_as_bson_datetime")]
    pub scheduled_at: Option<DateTime<Utc>>,

    /// Time at which the session was submitted to the infrastructure provider
    #[serde(with = "option_chrono_datetime_as_bson_datetime")]
    pub provisioned_at: Option<DateTime<Utc>>,

    /// Time at which the session reached an operational state
    #[serde(with = "option_chrono_datetime_as_bson_datetime")]
    pub operational_at: Option<DateTime<Utc>>,

    /// Time at which the session terminated
    #[serde(with = "option_chrono_datetime_as_bson_datetime")]
    pub terminated_at: Option<DateTime<Utc>>,

    /// Name as reported by the browser instance
    pub browser_name: Option<String>,

    /// Version as reported by the browser instance
    pub browser_version: Option<String>,

    /// Provisioner which deployed the session
    pub provisioner: Option<ProvisionerIdentifier>,

    /// Metadata provided by the provisioner
    pub provisioner_metadata: Option<ProvisionedSessionMetadata>,

    /// Metadata added by the client
    #[serde(default)]
    pub client_metadata: HashMap<String, String>,

    /// Number of bytes used by the video recording
    pub recording_bytes: Option<i64>,

    /// Reason why the session terminated
    pub termination: Option<SessionTerminationReason>,
}

impl SessionMetadata {
    /// Creates a new session metadata object without any values other than the primary key
    pub fn new(id: SessionIdentifier) -> Self {
        Self {
            id,
            created_at: None,
            scheduled_at: None,
            provisioned_at: None,
            operational_at: None,
            terminated_at: None,
            browser_name: None,
            browser_version: None,
            provisioner: None,
            provisioner_metadata: None,
            client_metadata: HashMap::new(),
            recording_bytes: None,
            termination: None,
        }
    }
}
