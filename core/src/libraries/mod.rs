//! Shared modules used by every service
//!
//! This module contains commonly used methods and data structures which are being used in individual services.
//! Each sub-module focuses on a specific area like `scheduling` or `storage`.
//! Small functions that don't belong anywhere else can be found in the `helpers` module.

pub mod helpers;
pub mod lifecycle;
pub mod metrics;
pub mod resources;
pub mod scheduling;
pub mod storage;

// TODO: Implement proper testing :)
// #[cfg(test)]
// pub mod testing;
