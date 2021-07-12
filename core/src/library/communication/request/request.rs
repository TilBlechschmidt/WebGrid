use super::super::event::Notification;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Describes a location where a response should be sent to
///
/// References a list data structure on which the requesting service can block on and
/// which may hold more than one reply. Carries a lower overhead than a Queue
/// but responses can only be consumed by one service (usually the requesting one).
/// Also cleans up after itself once all responses have been processed as opposed to a Queue
/// which requires explicit deletion.
pub type ResponseLocation = String;

/// Query for information which can be replied to
///
/// Note that Requests may not have side effects! Since the response
/// will not be acknowledged, it may get lost and the request
/// can and will get repeated eventually. Thus, in general, there must
/// be no side effects. If, for some sophisticated reason, side effects
/// are required (although they seldom are), they should be idempotent or
/// the response should not be considered important (e.g. it only serves as a confirmation
/// but nothing happens as a consequence of this confirmation — like with a user facing API
/// — in other words a situation where the request will not be repeated if the response is lost).
pub trait Request: Notification {
    /// Expected response type
    type Response: Serialize + DeserializeOwned + Debug + PartialEq;

    /// Location where a reply should be sent to
    fn reply_to(&self) -> ResponseLocation;
}
