use futures::lock::Mutex;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct RoutingInfo {
    /// Manager ID -> host
    pub managers: Arc<Mutex<HashMap<String, String>>>,
    /// Session ID -> host
    pub sessions: Arc<Mutex<HashMap<String, String>>>,
    /// Storage ID -> Provider ID -> Host
    pub storages: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
    /// API ID -> host
    pub apis: Arc<Mutex<HashMap<String, String>>>,
}

impl RoutingInfo {
    pub fn new() -> Self {
        RoutingInfo {
            managers: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            storages: Arc::new(Mutex::new(HashMap::new())),
            apis: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_manager_upstream(&self) -> Option<String> {
        let managers = self.managers.lock().await;

        managers.values().choose(&mut rand::thread_rng()).cloned()
    }

    pub async fn get_session_upstream(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    pub async fn get_storage_upstream(&self, storage_id: &str) -> Option<String> {
        let storages = self.storages.lock().await;

        storages
            .get(storage_id)
            .map(|providers| providers.values().choose(&mut rand::thread_rng()).cloned())
            .flatten()
    }

    pub async fn get_api_upstream(&self) -> Option<String> {
        let apis = self.apis.lock().await;

        apis.values().choose(&mut rand::thread_rng()).cloned()
    }

    // TODO Code duplication in the four methods below
    pub async fn add_manager_upstream(&self, manager_id: String, addr: &str) -> Option<String> {
        let mut managers = self.managers.lock().await;
        managers.insert(manager_id, addr.to_owned())
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

    pub async fn add_storage_upstream(
        &self,
        storage_id: &str,
        provider_id: &str,
        addr: &str,
    ) -> Option<String> {
        let mut storages = self.storages.lock().await;

        if !storages.contains_key(storage_id) {
            storages.insert(storage_id.to_owned(), HashMap::new());
        }

        // This will always be true due to the statement above! (because we locked it race conditions should be impossible)
        if let Some(storage) = storages.get_mut(storage_id) {
            storage.insert(provider_id.to_owned(), addr.to_owned())
        } else {
            unreachable!()
        }
    }

    pub async fn add_api_upstream(&self, api_id: &str, addr: &str) -> Option<String> {
        let mut apis = self.apis.lock().await;
        apis.insert(api_id.to_owned(), addr.to_owned())
    }

    pub async fn remove_manager_upstream(&self, manager_id: &str) {
        self.managers.lock().await.remove(manager_id);
    }

    pub async fn remove_session_upstream(&self, session_id: &str) {
        self.sessions.lock().await.remove(session_id);
    }

    pub async fn remove_storage_upstream(&self, storage_id: &str, provider_id: &str) {
        if let Some(storage) = self.storages.lock().await.get_mut(storage_id) {
            storage.remove(provider_id);
        }
    }

    pub async fn remove_api_upstream(&self, api_id: &str) {
        self.apis.lock().await.remove(api_id);
    }
}
