use crate::libraries::resources::{
    ResourceManager, ResourceManagerResult, StandaloneRedisResource,
};
use async_trait::async_trait;
use jatsl::TaskResourceHandle;
use lazy_static::lazy_static;
use std::{
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex as BlockingMutex},
    thread::sleep as threadSleep,
    time::Duration,
};

use super::lock_manager::LockManager;

// TODO Set this to num_cpus or a fork_count env variable
const DATABASES: usize = 16;
// TODO Set this to a random free port
const PORT: u16 = 40033;

lazy_static! {
    static ref REDIS_LOCK_MANAGER: LockManager<usize> = LockManager::new((0..DATABASES).collect());
    pub static ref TEST_RESOURCE_PROVIDER: TestResourceProvider = TestResourceProvider::new();
}

pub struct TestResourceProvider {
    redis_url: String,
    process: Arc<BlockingMutex<Child>>,
}

impl TestResourceProvider {
    pub fn new() -> Self {
        let process = Command::new("redis-server")
            .arg("--port")
            .arg(PORT.to_string())
            .arg("--databases")
            .arg(DATABASES.to_string())
            .arg("--save")
            .arg("")
            .arg("--appendonly")
            .arg("no")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        Self {
            redis_url: format!("redis://localhost:{}/", PORT),
            process: Arc::new(BlockingMutex::new(process)),
        }
    }

    pub fn bind_resource_manager(&self) -> TestResourceManager {
        let database_id = REDIS_LOCK_MANAGER.clone().request_lock();
        TestResourceManager {
            redis_url: self.redis_url.clone(),
            database_id,
        }
    }
}

impl Drop for TestResourceProvider {
    fn drop(&mut self) {
        // Give tests some grace period
        threadSleep(Duration::from_secs(1));
        self.process.lock().unwrap().kill().ok();
    }
}

impl Default for TestResourceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct TestResourceManager {
    redis_url: String,
    database_id: usize,
}

#[async_trait]
impl ResourceManager for TestResourceManager {
    type Redis = StandaloneRedisResource;
    type SharedRedis = Self::Redis;

    async fn redis(&self, handle: TaskResourceHandle) -> ResourceManagerResult<Self::Redis> {
        let mut con = StandaloneRedisResource::new(handle, &self.redis_url).await?;

        con.select(self.database_id).await?;
        con.set_logging(true);

        Ok(con)
    }

    async fn shared_redis(
        &self,
        handle: TaskResourceHandle,
    ) -> ResourceManagerResult<Self::SharedRedis> {
        self.redis(handle).await
    }
}

impl Drop for TestResourceManager {
    fn drop(&mut self) {
        // TODO Some RedisResource's might still live on after this thing has been dropped, leading to a database collision!
        // In reality this doesn't happen when the test harness is used, since it retains a reference for cleanup. Still something to consider!
        REDIS_LOCK_MANAGER.clone().return_lock(self.database_id);
    }
}
