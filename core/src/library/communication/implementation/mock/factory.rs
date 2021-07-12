use std::sync::Arc;

use super::discovery::MockServiceAdvertiser;
use super::response_collector::MockResponseCollector;
use super::response_publisher::MockResponsePublisher;
use super::{MockNotificationPublisher, MockQueueProvider, MockRequestor};
use crate::library::communication::event::{Notification, QueueDescriptorExtension};
use crate::library::communication::request::Request;
use crate::library::communication::CommunicationFactory;

pub struct MockCommunicationFactory {
    publisher: Arc<MockNotificationPublisher>,
    requestor: Arc<MockRequestor>,
}

impl CommunicationFactory for MockCommunicationFactory {
    type QueueProvider = MockQueueProvider;
    type NotificationPublisher = Arc<MockNotificationPublisher>;

    type Requestor = Arc<MockRequestor>;

    type ResponseCollector = MockResponseCollector;
    type ResponsePublisher = MockResponsePublisher;

    type ServiceAdvertiser = MockServiceAdvertiser;

    fn queue_provider(&self) -> Self::QueueProvider {
        MockQueueProvider {}
    }

    fn notification_publisher(&self) -> Self::NotificationPublisher {
        self.publisher.clone()
    }

    fn requestor(&self) -> Self::Requestor {
        self.requestor.clone()
    }

    fn response_collector(&self) -> Self::ResponseCollector {
        MockResponseCollector {}
    }

    fn response_publisher(&self) -> Self::ResponsePublisher {
        MockResponsePublisher {}
    }

    fn service_advertiser(&self) -> Self::ServiceAdvertiser {
        MockServiceAdvertiser {}
    }
}

impl Default for MockCommunicationFactory {
    fn default() -> Self {
        Self {
            publisher: Arc::new(MockNotificationPublisher::default()),
            requestor: Arc::new(MockRequestor::default()),
        }
    }
}

// Provide shorthands for the publisher / requestor methods
impl MockCommunicationFactory {
    pub fn expect_and_respond<R>(&self, request: &R, responses: Vec<R::Response>) -> &Self
    where
        R: Request + Send + Sync,
        R::Response: Send + Sync,
    {
        self.requestor.expect_and_respond(request, responses);
        self
    }

    pub fn expect<N: Notification + Send + Sync>(&self, notification: &N) -> &Self {
        self.publisher.expect(notification);
        self
    }

    pub fn expect_with_extension<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: QueueDescriptorExtension,
    ) -> &Self {
        self.publisher
            .expect_with_extension(notification, extension);
        self
    }
}
