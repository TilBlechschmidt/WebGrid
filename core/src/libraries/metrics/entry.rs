use hyper::http::{Method, StatusCode};
use std::{fmt, fmt::Display};

#[derive(Debug)]
pub enum SessionStatus {
    Queued,
    Pending,
    Alive,
    Terminated,
}

impl Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[derive(Debug)]
pub enum MetricsEntry {
    IncomingTraffic(u64),
    OutgoingTraffic(u64),
    RequestProcessed(Method, StatusCode),
    SessionStarted(f64),
    // TODO This needs to be used somewhere.
    SessionStatusChange(SessionStatus),
}
