//! Structures to realise a request-response pattern
//!
//! This module provides types for a request-response pattern where
//! requests are reliable and can be answered by multiple services. In contrast,
//! responses are not fully crash-tolerant and will get lost if the requesting
//! service crashes while processing it.
//!
//! However, this is by design as a crash in a service usually results in the
//! request (and thus response location) being lost as well. Despite this, it is
//! assumed that requests will usually be triggered while processing a [`Notification`](super::event::Notification)
//! which is inherently crash-resistant. Thus, when recovering, a service will repeat the
//! processing of the notification and in turn repeat the [`Request`]. For this reason,
//! requests may not have side effects.
//!
//! When talking about the request-response pattern, there are two parties involved:
//!
//! - Requesting side
//! - Responding side
//!
//! On the requesting side, a [`Requestor`] struct is used to send a [`Request`] and receive
//! the linked [`Request::Response`]. In its essence, it just wraps a [`NotificationPublisher`](super::event::NotificationPublisher)
//! (used to dispatch the request) and a [`ResponseCollector`] to wait for responses.
//!
//! On the responding side, a [`Responder`] struct is used to wait for incoming [`Requests`](Request),
//! by the means of implementing the [`Consumer`](super::event::Consumer) trait, process them using a
//! [`RequestProcessor`] and publishing the returned response using a [`ResponsePublisher`].

mod collector;
mod publisher;
#[allow(clippy::module_inception)]
mod request;
mod requestor;
mod responder;

pub use collector::*;
pub use publisher::*;
pub use request::*;
pub use requestor::*;
pub use responder::*;
