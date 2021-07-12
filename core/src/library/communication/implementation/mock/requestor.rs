use crate::library::communication::event::QueueDescriptor;
use crate::library::communication::request::{
    Request, RequestError, Requestor, ResponseCollectionTimeout,
};
use async_trait::async_trait;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct ExpectedRequest {
    serialized: Value,
    queue: QueueDescriptor,
    responses: Vec<Value>,
}

impl Default for MockRequestor {
    fn default() -> Self {
        MockRequestor {
            remaining: Arc::new(AtomicUsize::new(0)),
            expected: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

#[derive(Clone)]
pub struct MockRequestor {
    remaining: Arc<AtomicUsize>,
    expected: Arc<Mutex<VecDeque<ExpectedRequest>>>,
}

impl MockRequestor {
    pub fn expect_and_respond<R>(&self, request: &R, responses: Vec<R::Response>) -> &Self
    where
        R: Request + Send + Sync,
        R::Response: Send + Sync,
    {
        let serialized_request = serde_json::to_value(request).unwrap();
        let serialized_responses = responses
            .into_iter()
            .map(serde_json::to_value)
            .map(|r| r.unwrap())
            .collect();

        println!(
            "EXP REQ {} {}\n -> RES {:?}",
            R::queue().key(),
            serialized_request,
            serialized_responses
        );

        self.expected.lock().unwrap().push_back(ExpectedRequest {
            serialized: serialized_request,
            queue: R::queue(),
            responses: serialized_responses,
        });

        self.remaining.fetch_add(1, Ordering::SeqCst);
        self
    }
}

#[async_trait]
impl Requestor for Arc<MockRequestor> {
    async fn request<R>(
        &self,
        request: &R,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<Vec<R::Response>, RequestError>
    where
        R: Request + Send + Sync,
        R::Response: Send + Sync,
    {
        assert!(
            limit.is_some() || timeout != ResponseCollectionTimeout::None,
            "Calling `request` without a limit or timeout would block indefinitely!"
        );

        self.remaining.fetch_sub(1, Ordering::SeqCst);

        let serialized = serde_json::to_value(&request)
            .map_err(|e| RequestError::SendingFailure(Box::new(e)))?;

        println!("REQ {} {:?}", R::queue().key(), serialized);

        match self.expected.lock().unwrap().pop_front() {
            Some(expected) => {
                assert_eq!(
                    expected.queue,
                    R::queue(),
                    "Request queue (right) did not match expectation (left)"
                );

                let deserialized_expected: R = serde_json::from_value(expected.serialized)
                    .map_err(|e| RequestError::SendingFailure(Box::new(e)))?;

                assert_eq!(deserialized_expected, *request);

                let responses = expected
                    .responses
                    .into_iter()
                    .map(serde_json::from_value)
                    .map(|r| r.expect("Failed to deserialize response"))
                    .collect();

                Ok(responses)
            }
            None => panic!(
                "Received unexpected request on {}: {:?}",
                serialized,
                R::queue()
            ),
        }
    }
}

impl Drop for MockRequestor {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let remaining = self.remaining.load(Ordering::SeqCst);

            if remaining > 0 {
                panic!(
                    "MockRequestor was dropped with {} expected requests remaining",
                    remaining
                );
            }
        }
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use crate::library::communication::event::Notification;
    use crate::library::communication::request::ResponseLocation;
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
    struct MockResponse(usize);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct MockRequest(usize);

    impl Notification for MockRequest {
        fn queue() -> QueueDescriptor {
            QueueDescriptor::new("mock".into(), 42)
        }
    }

    impl Request for MockRequest {
        type Response = MockResponse;

        fn reply_to(&self) -> ResponseLocation {
            "somewhere".into()
        }
    }

    #[tokio::test]
    async fn fulfill_expectations() {
        let request = MockRequest(42);
        let response = MockResponse(42);
        let requestor = Arc::new(MockRequestor::default());

        requestor.expect_and_respond(&request, vec![response]);

        let responses = requestor
            .request(&request, Some(1), ResponseCollectionTimeout::None)
            .await
            .unwrap();

        assert_eq!(Some(&response), responses.first());
    }

    #[tokio::test]
    #[should_panic]
    async fn fail_on_different_content() {
        let expected = MockRequest(42);
        let actual = MockRequest(1337);
        let response = MockResponse(42);
        let requestor = Arc::new(MockRequestor::default());

        requestor.expect_and_respond(&expected, vec![response]);

        requestor
            .request(&actual, Some(1), ResponseCollectionTimeout::None)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn fail_on_unexpected() {
        let publisher = Arc::new(MockRequestor::default());

        publisher
            .request(&MockRequest(42), Some(1), ResponseCollectionTimeout::None)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn fail_on_missing() {
        MockRequestor::default().expect_and_respond(&MockRequest(42), vec![MockResponse(42)]);
    }
}
