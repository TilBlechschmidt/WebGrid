use sqlx::{pool::PoolConnection, sqlite::SqliteConnection, Transaction};
use std::{collections::VecDeque, path::PathBuf, sync::Arc};
use tokio::{
    fs::{read_dir, DirEntry, ReadDir},
    sync::{Mutex, Notify, Semaphore},
};

#[derive(Clone)]
pub struct FileSystemScanner {
    root: PathBuf,
    transaction: Arc<Mutex<Transaction<PoolConnection<SqliteConnection>>>>,
    stack: Arc<Mutex<VecDeque<PathBuf>>>,
    semaphore: Arc<Semaphore>,
    notify: Arc<Notify>,
}

impl FileSystemScanner {
    pub fn new(transaction: Transaction<PoolConnection<SqliteConnection>>, root: PathBuf) -> Self {
        Self {
            root,
            transaction: Arc::new(Mutex::new(transaction)),
            stack: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            semaphore: Arc::new(Semaphore::new(10)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn scan(self) -> Option<Transaction<PoolConnection<SqliteConnection>>> {
        // Clear previous indices
        {
            let mut con = self.transaction.lock().await;
            // TODO Get rid of unwrap!
            crate::database::delete_all_files(&mut (*con))
                .await
                .unwrap();
        }

        // Push the initial directory
        self.stack.lock().await.push_back(self.root.clone());

        // Set up some concurrency management
        let concurrency = self.semaphore.available_permits();
        let finish_notify = Arc::new(Notify::new());
        let mut handles = Vec::with_capacity(concurrency);

        // Spawn `concurrency` workers
        for _ in 0..concurrency {
            let scanner = self.clone();
            let finish_notify = finish_notify.clone();

            let handle = tokio::spawn(async move {
                loop {
                    if !scanner.next().await {
                        scanner.notify.notified().await;
                    }

                    // Bail if all processes are idle
                    if scanner.semaphore.available_permits() == concurrency {
                        finish_notify.notify();
                        return;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to become idle
        finish_notify.notified().await;

        // Release the scanner tasks from their notify wait
        for _ in 1..concurrency {
            self.notify.notify();
        }

        // Wait for all children to exit
        for handle in handles.into_iter() {
            handle.await.unwrap();
        }

        // Deconstruct `self` to gain access to the transaction within
        Arc::try_unwrap(self.transaction)
            .map(|i| i.into_inner())
            .ok()
    }

    async fn next(&self) -> bool {
        let _permit = self.semaphore.acquire().await;
        let path = self.stack.lock().await.pop_front();

        match path {
            Some(path) => {
                self.process_path(path).await;
                true
            }
            None => false,
        }
    }

    async fn process_path(&self, path: PathBuf) {
        match read_dir(&path).await {
            Err(e) => println!("Unable to read dir {:?} {:?}", path.display(), e),
            Ok(stream) => self.process_stream(stream).await,
        }
    }

    async fn process_stream(&self, mut stream: ReadDir) {
        while let Ok(entry) = stream.next_entry().await {
            match entry {
                None => break,
                Some(entry) => self.process_entry(entry).await,
            }
        }
    }

    async fn process_entry(&self, entry: DirEntry) {
        match entry.file_type().await {
            Err(e) => println!("Unable to get filetype for {:?} {:?}", entry, e),
            Ok(file_type) => {
                if file_type.is_dir() {
                    self.stack.lock().await.push_back(entry.path());
                    self.notify.notify();
                } else {
                    self.insert_file(entry).await;
                }
            }
        }
    }

    async fn insert_file(&self, entry: DirEntry) {
        let path_buf = entry.path();
        let path = path_buf.as_path();
        let path_str = path
            .strip_prefix(self.root.as_path())
            .unwrap()
            .to_str()
            .unwrap_or_default();

        let mut con = self.transaction.lock().await;
        crate::database::insert_file(path_str, entry.metadata().await.ok(), &mut (*con))
            .await
            .unwrap();
    }
}
