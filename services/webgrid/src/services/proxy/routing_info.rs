use futures::lock::Mutex;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct RoutingInfo {
    pub managers: Arc<Mutex<HashMap<String, String>>>,
    pub sessions: Arc<Mutex<HashMap<String, String>>>,
}

impl RoutingInfo {
    pub fn new() -> Self {
        RoutingInfo {
            managers: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_manager_upstreams(&self) -> Vec<String> {
        let managers = self.managers.lock().await;
        managers.iter().map(|(_, v)| v.clone()).collect()
    }

    pub async fn get_manager_upstream(&self) -> Option<String> {
        let upstreams = self.get_manager_upstreams().await;
        if upstreams.is_empty() {
            return None;
        }
        upstreams.choose(&mut rand::thread_rng()).cloned()
    }

    pub async fn get_session_upstream(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    // TODO Code duplication in the four methods below
    pub async fn add_manager_upstream(
        &self,
        manager_id: String,
        host: &str,
        port: &str,
    ) -> Option<String> {
        let addr = format!("{}:{}", host, port);
        let mut managers = self.managers.lock().await;
        managers.insert(manager_id, addr)
    }

    pub async fn add_session_upstream(
        &self,
        session_id: String,
        host: &str,
        port: &str,
    ) -> Option<String> {
        let addr = format!("{}:{}", host, port);
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id, addr)
    }

    pub async fn remove_manager_upstream(&self, manager_id: &str) {
        self.managers.lock().await.remove(manager_id);
    }

    pub async fn remove_session_upstream(&self, session_id: &str) {
        self.sessions.lock().await.remove(session_id);
    }
}
