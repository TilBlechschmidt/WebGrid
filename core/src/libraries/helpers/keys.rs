//! Redis database keys
//!
//! This module contains a list of functions to generate keys for accessing values in the Redis database.
//! In the future this will be extracted to the constants file.
//!
//! The keys are stored in submodules organised by components.

macro_rules! static_keys {
    ($($name:ident = $value:expr;)+) => {
        lazy_static::lazy_static! {
            $(
                pub static ref $name: String = $value;
            )+
        }
    };
}

static_keys! {
    HEARTBEAT_REFRESH_CHANNEL = "heartbeat-refresh".to_string();
}

pub mod orchestrator {
    static_keys! {
        LIST = "orchestrators".to_string();
    }

    fn orchestrator_prefix(orchestrator_id: &str) -> String {
        format!("orchestrator:{}", orchestrator_id)
    }

    pub fn metadata(orchestrator_id: &str) -> String {
        orchestrator_prefix(orchestrator_id)
    }

    pub fn backlog(orchestrator_id: &str) -> String {
        format!("{}:backlog", orchestrator_prefix(orchestrator_id))
    }

    pub fn pending(orchestrator_id: &str) -> String {
        format!("{}:pending", orchestrator_prefix(orchestrator_id))
    }

    pub fn heartbeat(orchestrator_id: &str) -> String {
        format!("{}:heartbeat", orchestrator_prefix(orchestrator_id))
    }

    pub fn retain(orchestrator_id: &str) -> String {
        format!("{}:retain", orchestrator_prefix(orchestrator_id))
    }

    pub mod capabilities {
        use super::orchestrator_prefix;

        pub fn platform_name(orchestrator_id: &str) -> String {
            format!(
                "{}:capabilities:platformName",
                orchestrator_prefix(orchestrator_id)
            )
        }

        pub fn browsers(orchestrator_id: &str) -> String {
            format!(
                "{}:capabilities:browsers",
                orchestrator_prefix(orchestrator_id)
            )
        }
    }

    pub mod slots {
        use super::orchestrator_prefix;

        pub fn allocated(orchestrator_id: &str) -> String {
            format!("{}:slots", orchestrator_prefix(orchestrator_id))
        }

        pub fn available(orchestrator_id: &str) -> String {
            format!("{}:slots.available", orchestrator_prefix(orchestrator_id))
        }

        pub fn reclaimed(orchestrator_id: &str) -> String {
            format!("{}:slots.reclaimed", orchestrator_prefix(orchestrator_id))
        }
    }
}

pub mod manager {
    fn manager_prefix(manager_id: &str) -> String {
        format!("manager:{}", manager_id)
    }

    pub fn host(manager_id: &str) -> String {
        format!("{}:host", manager_prefix(manager_id))
    }
}

pub mod session {
    static_keys! {
        LIST_ACTIVE = "sessions.active".to_string();
        LIST_TERMINATED = "sessions.terminated".to_string();
    }

    fn session_prefix(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    pub fn status(session_id: &str) -> String {
        format!("{}:status", session_prefix(session_id))
    }

    pub fn capabilities(session_id: &str) -> String {
        format!("{}:capabilities", session_prefix(session_id))
    }

    pub fn metadata(session_id: &str) -> String {
        format!("{}:metadata", session_prefix(session_id))
    }

    pub fn upstream(session_id: &str) -> String {
        format!("{}:upstream", session_prefix(session_id))
    }

    pub fn downstream(session_id: &str) -> String {
        format!("{}:downstream", session_prefix(session_id))
    }

    pub fn slot(session_id: &str) -> String {
        format!("{}:slot", session_prefix(session_id))
    }

    pub fn orchestrator(session_id: &str) -> String {
        format!("{}:orchestrator", session_prefix(session_id))
    }

    pub fn storage(session_id: &str) -> String {
        format!("{}:storage", session_prefix(session_id))
    }

    pub mod telemetry {
        use super::session_prefix;

        pub fn creation(session_id: &str) -> String {
            format!("{}:telemetry.creation", session_prefix(session_id))
        }
    }

    pub mod heartbeat {
        use super::session_prefix;

        pub fn manager(session_id: &str) -> String {
            format!("{}:heartbeat.manager", session_prefix(session_id))
        }

        pub fn node(session_id: &str) -> String {
            format!("{}:heartbeat.node", session_prefix(session_id))
        }
    }
}

pub mod storage {
    fn storage_prefix(storage_id: &str) -> String {
        format!("storage:{}", storage_id)
    }

    pub fn host(storage_id: &str, provider_id: &str) -> String {
        format!("{}:{}:host", storage_prefix(storage_id), provider_id)
    }

    pub mod metadata {
        use super::storage_prefix;

        pub fn pending(storage_id: &str) -> String {
            format!("{}:metadata.pending", storage_prefix(storage_id))
        }
    }
}

pub mod metrics {
    pub mod http {
        static_keys! {
            NET_BYTES_TOTAL = "metrics:http:net.bytes.total".to_string();
        }

        pub fn requests_total(method: &str) -> String {
            format!("metrics:http:requestsTotal:{}", method)
        }
    }

    pub mod session {
        pub fn log(level: &str) -> String {
            format!("metrics:sessions:log:{}", level)
        }

        pub mod startup_histogram {
            static_keys! {
                BUCKETS = "metrics:sessions:startup.histogram:buckets".to_string();
                SUM = "metrics:sessions:startup.histogram:sum".to_string();
                COUNT = "metrics:sessions:startup.histogram:count".to_string();
            }
        }
    }

    pub mod storage {
        static_keys! {
            CAPACITY = "metrics:storage:disk.bytes.total".to_string();
            USAGE = "metrics:storage:disk.bytes.used".to_string();
        }
    }
}

pub mod api {
    fn api_prefix(api_id: &str) -> String {
        format!("api:{}", api_id)
    }

    pub fn host(api_id: &str) -> String {
        format!("{}:host", api_prefix(api_id))
    }
}
