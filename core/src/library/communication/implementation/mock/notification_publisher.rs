use super::ExpectationMode;
use crate::library::communication::event::{
    Notification, NotificationPublisher, QueueDescriptor, QueueDescriptorExtension,
};
use crate::library::EmptyResult;
use async_trait::async_trait;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use std::any::type_name;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct ExpectedNotification {
    serialized: String,
    queue: QueueDescriptor,
    extension: Option<QueueDescriptorExtension>,
}

impl ExpectedNotification {
    fn deserialized<'a, D: Deserialize<'a>>(&'a self) -> Result<D, String> {
        let message = format!(
            "failed to deserialize expected value to type {}: {}",
            type_name::<D>(),
            self.serialized
        );
        serde_json::from_str(&self.serialized).map_err(|_| message)
    }
}

impl Default for MockNotificationPublisher {
    fn default() -> Self {
        Self {
            remaining: AtomicUsize::new(0),
            expected: Mutex::new(VecDeque::new()),
            mode: ExpectationMode::ExpectOnlyProvided,
        }
    }
}

pub struct MockNotificationPublisher {
    remaining: AtomicUsize,
    expected: Mutex<VecDeque<ExpectedNotification>>,
    mode: ExpectationMode,
}

#[async_trait]
impl NotificationPublisher for Arc<MockNotificationPublisher> {
    async fn publish<N: Notification + Send + Sync>(&self, notification: &N) -> EmptyResult {
        self.handle(notification, None).await;
        Ok(())
    }

    async fn publish_with_extension<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: QueueDescriptorExtension,
    ) -> EmptyResult {
        self.handle(notification, Some(extension)).await;
        Ok(())
    }
}

impl MockNotificationPublisher {
    #[allow(clippy::field_reassign_with_default)]
    pub fn permitting_noise() -> Self {
        let mut instance = Self::default();
        instance.mode = ExpectationMode::AllowNoise;
        instance
    }

    pub fn expect<N: Notification + Send + Sync>(&self, notification: &N) -> &Self {
        self.add_expectation(notification, None).unwrap();
        self
    }

    pub fn expect_with_extension<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: QueueDescriptorExtension,
    ) -> &Self {
        self.add_expectation(notification, Some(extension)).unwrap();
        self
    }

    fn add_expectation<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: Option<QueueDescriptorExtension>,
    ) -> EmptyResult {
        let serialized = serde_json::to_string(notification)?;

        let queue = N::queue();
        let queue_key = match &extension {
            Some(ext) => queue.key_with_extension(ext),
            None => queue.key().to_string(),
        };

        println!("EXP {} {}", queue_key, serialized);

        self.expected
            .lock()
            .unwrap()
            .push_back(ExpectedNotification {
                serialized,
                queue,
                extension,
            });

        self.remaining.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    async fn handle<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: Option<QueueDescriptorExtension>,
    ) {
        let queue = N::queue();
        let queue_key = match &extension {
            Some(ext) => queue.key_with_extension(ext),
            None => queue.key().to_string(),
        };

        let json = serde_json::to_string(&notification)
            .expect("Published value failed to convert to JSON");
        println!("PUB {} {}", queue_key, json);

        match self.mode {
            ExpectationMode::Ignore => {}
            ExpectationMode::ExpectOnlyProvided => {
                match self.expected.lock().unwrap().pop_front() {
                    None => panic!(
                        "Unexpected notification was published to {:?}: {:?}",
                        queue_key, json
                    ),
                    Some(expected) => {
                        assert_eq!(
                            expected.queue, queue,
                            "Notification queue (right) did not match expectation (left)"
                        );
                        assert_eq!(
                            expected.extension, extension,
                            "Notification queue extension (right) did not match expectation (left)"
                        );
                        assert_eq!(expected.deserialized::<N>().unwrap(), *notification);
                    }
                }
            }
            ExpectationMode::AllowNoise => {
                let mut lock = self.expected.lock().unwrap();
                if let Some(expected) = lock.front() {
                    if expected.queue == queue && expected.extension == extension {
                        if let Ok(expected_notification) = expected.deserialized::<N>() {
                            if expected_notification == *notification {
                                lock.pop_front();
                            }
                        }
                    }
                }
            }
        };

        let new_length = self.expected.lock().unwrap().len();
        self.remaining.store(new_length, Ordering::SeqCst);
    }
}

impl Drop for MockNotificationPublisher {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let remaining = self.remaining.load(Ordering::SeqCst);

            if self.mode != ExpectationMode::Ignore && remaining > 0 {
                panic!(
                    "MockNotificationPublisher was dropped with {} expected notifications remaining",
                    remaining
                );
            }
        }
    }
}

mod does {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct MockNotification(usize);

    impl Notification for MockNotification {
        fn queue() -> QueueDescriptor {
            QueueDescriptor::new("mock".into(), 42)
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ComplexMockNotification {
        answer: usize,
        other_field: String,
    }

    impl Notification for ComplexMockNotification {
        fn queue() -> QueueDescriptor {
            QueueDescriptor::new("mock".into(), 42)
        }
    }

    #[tokio::test]
    async fn fulfill_expectations() {
        let notification = MockNotification(42);
        let publisher = Arc::new(MockNotificationPublisher::default());

        publisher.expect(&notification);
        publisher.publish(&notification).await.unwrap();
    }

    #[tokio::test]
    async fn allow_noise() {
        let notification = MockNotification(42);
        let noise = MockNotification(1337);
        let publisher = Arc::new(MockNotificationPublisher::permitting_noise());

        publisher.expect(&notification);
        publisher.publish(&noise).await.unwrap();
        publisher.publish(&notification).await.unwrap();
        publisher.publish(&noise).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn fail_on_different_content() {
        let expected = ComplexMockNotification {
            answer: 42,
            other_field: "hello world".into(),
        };

        let actual = ComplexMockNotification {
            answer: 42,
            other_field: "hello john".into(),
        };

        let publisher = Arc::new(MockNotificationPublisher::default());

        publisher.expect(&expected);
        publisher.publish(&actual).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn fail_on_unexpected() {
        let publisher = Arc::new(MockNotificationPublisher::default());
        publisher.publish(&MockNotification(42)).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn fail_on_missing() {
        MockNotificationPublisher::default().expect(&MockNotification(42));
    }
}
