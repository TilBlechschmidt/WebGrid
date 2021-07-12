//! Structures to realise an event-driven service architecture
//!
//! In an event driven world, services have no knowledge of each other.
//! Each service operates independently and during the operation, certain
//! events occur. For each of these (that are of relevance to other services)
//! an event [`Notification`] is published.
//! Every interested party may then subscribe to the [`Queue`](QueueDescriptor)
//! of notifications for a specific event and process them. While processing,
//! it is common that other events are triggered and thus further notifications
//! published.
//!
//! Additionally, notifications are consumed in a reliable and resilient way using
//! a concept called [`ConsumerGroups`](ConsumerGroupDescriptor). Instead of using
//! simple publish subscribe between all connected services, messages are stored in
//! a log-like data structure (usually of limited length where old elements are evicted).
//!
//! When reading from this data structure, services may define a [`QueueLocation`] from
//! which they want to begin processing. All notifications have to be acknowledged once
//! processing concludes. Upon crashing, the [`Consumer`](ConsumerIdentifier) may then
//! resume from the last acknowledged notification. This ensures that no [`QueueEntries`](QueueEntry)
//! are left unprocessed.
//!
//! The last benefit of this data structure is that multiple [`Consumers`](ConsumerIdentifier) may
//! share a [`ConsumerGroup`](ConsumerGroupDescriptor). All participants in a group then collectively
//! process the incoming notification stream where each notification is assigned to only one consumer
//! within the group (effectively implementing load balancing and simple, dynamic scalability).

mod consumer;
mod consumer_group;
mod notification;
mod publisher;
mod queue;
mod queue_provider;

pub use consumer::*;
pub use consumer_group::*;
pub use notification::*;
pub use publisher::*;
pub use queue::*;
pub use queue_provider::*;
