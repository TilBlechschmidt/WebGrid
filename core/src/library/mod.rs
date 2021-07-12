//! Independent and project agnostic libraries
//!
//! Ideally, any of the library submodules in this module can be extracted into their own crate
//! at any given time (like it happened with [`jatsl`](https://docs.rs/jatsl)). Libraries
//! in this module have been developed with WebGrid in mind and are powering core functionalities,
//! however, they are in no way bound to the project and everything domain specific has been
//! extracted into the [`domain`](super::domain) module.

pub mod communication;
pub mod helpers;
pub mod http;
#[cfg(feature = "storage")]
pub mod storage;

/// Generic error type
pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Result with no value and a [`BoxedError`]
pub type EmptyResult = Result<(), BoxedError>;
