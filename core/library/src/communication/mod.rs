//! Structures to communication between services in a distributed system
//!
//! TODO: This doc-block is redundant and should be replaced with something more descriptive for what this module actually encompasses.
//!
//! In general, there are two modes of operation:
//!
//! 1. Publish and subscribe
//! 2. Request and response
//!
//! The first is used primarily for event notifications which make up the
//! event-driven architecture. Whenever something noteworthy happens in the
//! system, a notification describing what happened will be published.
//! The notification data structure implements the [`Notification`](event::Notification) trait and
//! thus describes where to expect it in a type-safe manner.
//! In this mode, everybody can publish notifications and all interested parties
//! can listen in and react to published event notifications. For more details and
//! a more in-depth explanation, consult the [`event`] module.
//!
//! The second mode of operation is request and response. In a simple
//! scenario, a component may request information from another one and thus
//! one service may receive one response. However, it is also possible for
//! multiple respondents to send a response as more than one service may
//! serve the request simultaneously (if desired). This is implemented using
//! the [`Request`](request::Request) trait which builds on the [`Notification`](event::Notification) trait by adding
//! a response channel.

mod communication_factory;
mod error;

pub mod discovery;
pub mod event;
pub mod implementation;
pub mod request;

pub use communication_factory::CommunicationFactory;
pub use error::BlackboxError;
