//! File system accessors
//!
//! This module is responsible for accessing, managing and cleaning up the file system.

mod database;
mod scan;
mod storage_handler;

pub use self::storage_handler::{StorageError, StorageHandler};
