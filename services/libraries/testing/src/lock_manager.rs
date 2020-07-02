use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct LockManager<R> {
    resources: Arc<(Mutex<Vec<R>>, Condvar)>,
}

impl<R> LockManager<R> {
    pub fn new(resources: Vec<R>) -> Self {
        Self {
            resources: Arc::new((Mutex::new(resources), Condvar::new())),
        }
    }

    pub fn request_lock(&self) -> R {
        let (lock, cvar) = &*self.resources;
        let mut slots = lock.lock().unwrap();

        while slots.is_empty() {
            slots = cvar.wait(slots).unwrap();
        }

        slots.pop().unwrap()
    }

    pub fn return_lock(&self, resource: R) {
        let (lock, cvar) = &*self.resources;
        let mut slots = lock.lock().unwrap();

        slots.push(resource);
        cvar.notify_one();
    }
}
