//! Structures for logging to database

use log::info;
use redis::{aio::ConnectionLike, cmd, AsyncCommands, RedisResult};
use std::fmt;

/// Database logging facility
pub struct Logger<C: ConnectionLike> {
    con: C,
    component: String,
}

// Initializer
impl<C: ConnectionLike> Logger<C> {
    /// Creates a new database logger for the specified component
    // TODO This field should be called service instead of component
    pub fn new(con: C, component: String) -> Logger<C> {
        Logger { con, component }
    }
}

// Logging functions
impl<C: ConnectionLike + AsyncCommands> Logger<C> {
    /// Write a raw log message to the database
    #[rustfmt::skip]
    async fn log_raw(
        &mut self,
        session_id: &str,
        level: LogLevel,
        code: String,
        meta: Option<String>,
    ) -> RedisResult<()> {
        let key = format!("session:{}:log", session_id);
        let metrics_key = format!("metrics:sessions:log:{:?}", level);

        info!("Writing log code {} for {}", code, session_id);
        self.con.hincr::<_, _, _, ()>(metrics_key, &code, 1).await.ok();

        cmd("XADD")
            .arg(key).arg("*")
            .arg("component").arg(&self.component)
            .arg("level").arg(level.to_string())
            .arg("code").arg(code)
            .arg("meta").arg(meta.unwrap_or_else(|| "{}".to_string()))
            .query_async(&mut self.con)
            .await
    }

    /// Write a log message to the database
    pub async fn log(
        &mut self,
        session_id: &str,
        code: LogCode,
        meta: Option<String>,
    ) -> RedisResult<()> {
        self.log_raw(session_id, code.level(), code.to_string(), meta)
            .await
    }
}

/// Wrapper around logger that stores the session_id
pub struct SessionLogger<C: ConnectionLike + AsyncCommands> {
    logger: Logger<C>,
    session_id: String,
}

impl<C: ConnectionLike + AsyncCommands> SessionLogger<C> {
    /// Creates a new database logger for the specified component and session
    pub fn new(con: C, component: String, session_id: String) -> SessionLogger<C> {
        SessionLogger {
            logger: Logger::new(con, component),
            session_id,
        }
    }

    /// Write a log message to the database
    pub async fn log(&mut self, code: LogCode, meta: Option<String>) -> RedisResult<()> {
        self.logger
            .log_raw(&self.session_id, code.level(), code.to_string(), meta)
            .await
    }
}

/// Message criticality
/// - **INFO** used for status reports
/// - **WARN** used for recoverable errors
/// - **FAIL** used for unrecoverable errors
#[derive(Debug)]
pub enum LogLevel {
    Info,
    Warn,
    Fail,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Log event types
#[derive(Debug)]
pub enum LogCode {
    // Generic
    // -- Fail
    Failure,

    // Node
    // -- Info
    /// node has become active
    Boot,
    /// driver in startup
    DStart,
    /// driver has become responsive
    DAlive,
    /// local session created
    LsInit,
    /// session closed by downstream client
    Closed,
    /// node enters shutdown
    Halt,
    // -- Fail
    /// driver has not become responsive
    DTimeout,
    /// driver process reported an error
    DFailure,
    /// session has been inactive too long
    STimeout,
    /// node terminates due to fault condition
    Term,

    // Orchestrator
    // -- Info
    /// node is being scheduled for startup
    Sched,
    // -- Fail
    /// creation/startup failure
    StartFail,

    // Manager
    // -- Info
    /// session has been queued at orchestrators
    Queued,
    /// node slot has been allocated
    NAlloc,
    /// awaiting node startup
    Pending,
    /// node has become responsive, client served
    NAlive,
    // -- Warn
    /// client left before scheduling completed
    CLeft,
    // -- Fail
    /// invalid capabilities requested
    InvalidCap,
    /// no orchestrator can satisfy the capabilities
    QUnavailable,
    /// timed out waiting in queue
    QTimeout,
    /// timed out waiting for orchestrator to schedule node
    OTimeout,
    /// timed out waiting for node to become responsive
    NTimeout,
    // Proxy
}

impl LogCode {
    /// Log level for a given event type
    pub fn level(&self) -> LogLevel {
        match self {
            // Generic
            LogCode::Failure => LogLevel::Fail,

            // Node
            LogCode::Boot => LogLevel::Info,
            LogCode::DStart => LogLevel::Info,
            LogCode::DAlive => LogLevel::Info,
            LogCode::LsInit => LogLevel::Info,
            LogCode::Closed => LogLevel::Info,
            LogCode::Halt => LogLevel::Info,

            LogCode::DTimeout => LogLevel::Fail,
            LogCode::DFailure => LogLevel::Fail,
            LogCode::STimeout => LogLevel::Fail,
            LogCode::Term => LogLevel::Fail,

            // Orchestrator
            LogCode::Sched => LogLevel::Info,
            LogCode::StartFail => LogLevel::Fail,

            // Manager
            LogCode::InvalidCap => LogLevel::Fail,
            LogCode::QUnavailable => LogLevel::Fail,
            LogCode::Queued => LogLevel::Info,
            LogCode::NAlloc => LogLevel::Info,
            LogCode::Pending => LogLevel::Info,
            LogCode::NAlive => LogLevel::Info,

            LogCode::CLeft => LogLevel::Warn,

            LogCode::QTimeout => LogLevel::Fail,
            LogCode::OTimeout => LogLevel::Fail,
            LogCode::NTimeout => LogLevel::Fail,
        }
    }
}

impl fmt::Display for LogCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
