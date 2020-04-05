use redis::{aio::MultiplexedConnection, cmd, RedisResult};
use std::fmt;
use log::info;

pub struct Logger {
    con: MultiplexedConnection,
    component: String,
}

// Initializer
impl Logger {
    pub fn new(con: &MultiplexedConnection, component: String) -> Logger {
        pretty_env_logger::init_timed();
        
        Logger {
            con: con.clone(),
            component,
        }
    }
}

// Logging functions
impl Logger {
    #[rustfmt::skip]
    async fn log_raw(
        &self,
        session_id: &str,
        level: LogLevel,
        code: String,
        meta: Option<String>,
    ) -> RedisResult<()> {
        let mut con = self.con.clone();
        let key = format!("stream:{}:log", session_id);

        info!("Writing log code {} for {}", code, session_id);

        cmd("XADD")
            .arg(key).arg("*")
            .arg("component").arg(&self.component)
            .arg("level").arg(level.to_string())
            .arg("code").arg(code)
            .arg("meta").arg(meta.unwrap_or_else(|| "{}".to_string()))
            .query_async(&mut con)
            .await
    }

    pub async fn log(
        &self,
        session_id: &str,
        code: LogCode,
        meta: Option<String>,
    ) -> RedisResult<()> {
        self.log_raw(session_id, code.level(), code.to_string(), meta)
            .await
    }
}

pub struct SessionLogger {
    logger: Logger,
    session_id: String,
}

impl SessionLogger {
    pub fn new(
        con: &MultiplexedConnection,
        component: String,
        session_id: String,
    ) -> SessionLogger {
        SessionLogger {
            logger: Logger::new(con, component),
            session_id,
        }
    }

    pub async fn log(&self, code: LogCode, meta: Option<String>) -> RedisResult<()> {
        self.logger
            .log_raw(&self.session_id, code.level(), code.to_string(), meta)
            .await
    }
}

#[derive(Debug)]
pub enum LogLevel {
    INFO,
    WARN,
    FAIL,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum LogCode {
    // Generic
    // -- Fail
    FAILURE,

    // Node
    // -- Info
    BOOT,
    DSTART,
    DALIVE,
    LSINIT,
    CLOSED,
    HALT,
    // -- Fail
    DTIMEOUT,
    DFAILURE,
    STIMEOUT,
    TERM,

    // Orchestrator
    // -- Info
    SCHED,
    // -- Fail
    STARTFAIL,

    // Manager
    // -- Info
    QUEUED,
    NALLOC,
    PENDING,
    NALIVE,
    // -- Warn
    CLEFT,
    // -- Fail
    QUNAVAILABLE,
    QTIMEOUT,
    OTIMEOUT,
    NTIMEOUT,
    // Proxy
}

impl LogCode {
    pub fn level(&self) -> LogLevel {
        match self {
            // Generic
            LogCode::FAILURE => LogLevel::FAIL,

            // Node
            LogCode::BOOT => LogLevel::INFO,
            LogCode::DSTART => LogLevel::INFO,
            LogCode::DALIVE => LogLevel::INFO,
            LogCode::LSINIT => LogLevel::INFO,
            LogCode::CLOSED => LogLevel::INFO,
            LogCode::HALT => LogLevel::INFO,

            LogCode::DTIMEOUT => LogLevel::FAIL,
            LogCode::DFAILURE => LogLevel::FAIL,
            LogCode::STIMEOUT => LogLevel::FAIL,
            LogCode::TERM => LogLevel::FAIL,

            // Orchestrator
            LogCode::SCHED => LogLevel::INFO,
            LogCode::STARTFAIL => LogLevel::FAIL,

            // Manager
            LogCode::QUNAVAILABLE => LogLevel::FAIL,
            LogCode::QUEUED => LogLevel::INFO,
            LogCode::NALLOC => LogLevel::INFO,
            LogCode::PENDING => LogLevel::INFO,
            LogCode::NALIVE => LogLevel::INFO,

            LogCode::CLEFT => LogLevel::WARN,

            LogCode::QTIMEOUT => LogLevel::FAIL,
            LogCode::OTIMEOUT => LogLevel::FAIL,
            LogCode::NTIMEOUT => LogLevel::FAIL,
        }
    }
}

impl fmt::Display for LogCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
