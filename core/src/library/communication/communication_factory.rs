use crate::library::communication::event::{NotificationPublisher, QueueProvider};
use crate::library::communication::request::{Requestor, ResponseCollector, ResponsePublisher};

use super::discovery::ServiceAdvertiser;

/// Factory to provide implementations for the traits from this module
pub trait CommunicationFactory {
    /// [`QueueProvider`] implementation type
    type QueueProvider: QueueProvider + Send + Sync;
    /// [`NotificationPublisher`] implementation type
    type NotificationPublisher: NotificationPublisher + Send + Sync;

    /// [`Requestor`] implementation type
    type Requestor: Requestor + Send + Sync;

    /// [`ResponseCollector`] implementation type
    type ResponseCollector: ResponseCollector + Send + Sync;
    /// [`ResponsePublisher`] implementation type
    type ResponsePublisher: ResponsePublisher + Send + Sync;

    /// [`ServiceAdvertiser`] implementation type
    type ServiceAdvertiser: ServiceAdvertiser + Send + Sync;

    /// Instantiates a new [`QueueProvider`]
    fn queue_provider(&self) -> Self::QueueProvider;
    /// Instantiates a new [`NotificationPublisher`]
    fn notification_publisher(&self) -> Self::NotificationPublisher;

    /// Instantiates a new [`Requestor`]
    fn requestor(&self) -> Self::Requestor;

    /// Instantiates a new [`ResponseCollector`]
    fn response_collector(&self) -> Self::ResponseCollector;
    /// Instantiates a new [`ResponsePublisher`]
    fn response_publisher(&self) -> Self::ResponsePublisher;

    /// Instantiates a new [`ServiceAdvertiser`]
    fn service_advertiser(&self) -> Self::ServiceAdvertiser;
}
