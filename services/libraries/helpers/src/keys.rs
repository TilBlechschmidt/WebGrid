macro_rules! static_keys {
    ($($name:ident = $value:expr;)+) => {
        lazy_static::lazy_static! {
            $(
                pub static ref $name: String = $value;
            )+
        }
    };
}

pub mod orchestrator {
    static_keys! {
        LIST = "orchestrators".to_string();
        HEARTBEAT = format!("orchestrator:{}:heartbeat", *crate::env::service::orchestrator::ID);
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
    static_keys! {
        LIST = "managers".to_string();
        HEARTBEAT = format!("manager:{}:heartbeat", *crate::env::service::manager::ID);
        METADATA = format!("manager:{}", *crate::env::service::manager::ID);
    }
}

pub mod session {
    static_keys! {
        LIST_ACTIVE = "sessions.active".to_string();
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
