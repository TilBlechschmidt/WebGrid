use super::super::GqlContext;
use chrono::{DateTime, Utc};
use domain::event::ProvisionerIdentifier;
use domain::SessionMetadata;
use juniper::{graphql_object, GraphQLEnum, GraphQLObject};

#[derive(GraphQLObject)]
pub struct MetadataEntry {
    key: String,
    value: String,
}

/// List of times at which the session passed certain state transitions
#[derive(GraphQLObject)]
pub struct Timestamps {
    /// Time at which the session object was initially created
    pub created_at: Option<DateTime<Utc>>,

    /// Time at which the session was scheduled with a provisioner
    pub scheduled_at: Option<DateTime<Utc>>,

    /// Time at which the session was submitted to the infrastructure provider
    pub provisioned_at: Option<DateTime<Utc>>,

    /// Time at which the session reached an operational state
    pub operational_at: Option<DateTime<Utc>>,

    /// Time at which the session terminated
    pub terminated_at: Option<DateTime<Utc>>,
}

/// Metadata provided by the browser on startup
#[derive(GraphQLObject)]
pub struct Browser {
    name: String,
    version: String,
}

/// Values identifying the provisioner used by the session
#[derive(GraphQLObject)]
pub struct Provisioner {
    id: ProvisionerIdentifier,
}

/// Metadata attached to the session
#[derive(GraphQLObject)]
pub struct Metadata {
    /// Values provided by a client either at startup or during the session
    client: Vec<MetadataEntry>,
    /// Values attached by the provisioner
    provisioner: Vec<MetadataEntry>,
}

/// Details of the video recording
#[derive(GraphQLObject)]
pub struct Video {
    /// HLS m3u8 playlist location
    playlist: String,
    /// Total number of bytes excluding metadata
    size: i32,
}

/// Lifecycle position of a session
#[derive(GraphQLEnum, Debug)]
pub enum SessionState {
    /// Submitted by a client
    Created,
    /// Assigned to a provisioner
    Scheduled,
    /// Processed by the assigned provisioner and handed over to the infrastructure
    Provisioned,
    /// Up and running, ready to serve requests
    Operational,
    /// Completely shut down and no longer serving requests. Either due to explicit shutdown by client or due to crash.
    Terminated,
}

impl SessionState {
    /// Returns the database keys to use when querying for each state
    pub fn database_key(&self) -> &str {
        match self {
            SessionState::Created => "createdAt",
            SessionState::Scheduled => "scheduledAt",
            SessionState::Provisioned => "provisionedAt",
            SessionState::Operational => "operationalAt",
            SessionState::Terminated => "terminatedAt",
        }
    }
}

pub struct Session {
    metadata: SessionMetadata,
}

impl Session {
    pub fn new(metadata: SessionMetadata) -> Self {
        Self { metadata }
    }
}

#[graphql_object(context = GqlContext)]
impl Session {
    /// Unique identifier generated on creation
    fn id(&self) -> String {
        self.metadata.id.to_string()
    }

    fn video(&self) -> Video {
        Video {
            playlist: format!("/storage/{}/screen.m3u8", &self.metadata.id),
            size: self.metadata.recording_bytes.unwrap_or_default() as i32,
        }
    }

    /// Can be empty when the session has not yet been provisioned.
    fn provisioner(&self) -> Option<Provisioner> {
        self.metadata
            .provisioner
            .clone()
            .map(|id| Provisioner { id })
    }

    fn metadata(&self) -> Metadata {
        let client = self
            .metadata
            .client_metadata
            .iter()
            .map(|(k, v)| MetadataEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        let provisioner = self
            .metadata
            .provisioner_metadata
            .as_ref()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| MetadataEntry {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Metadata {
            client,
            provisioner,
        }
    }

    fn timestamps(&self) -> Timestamps {
        Timestamps {
            created_at: self.metadata.created_at,
            scheduled_at: self.metadata.scheduled_at,
            provisioned_at: self.metadata.provisioned_at,
            operational_at: self.metadata.operational_at,
            terminated_at: self.metadata.terminated_at,
        }
    }

    /// Can be empty if the browser has not yet been started.
    fn browser(&self) -> Option<Browser> {
        if let (Some(name), Some(version)) = (
            self.metadata.browser_name.clone(),
            self.metadata.browser_version.clone(),
        ) {
            Some(Browser { name, version })
        } else {
            None
        }
    }
}
