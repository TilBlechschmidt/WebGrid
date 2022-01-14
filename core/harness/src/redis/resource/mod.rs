#![allow(dead_code)]

use super::handle::{HandleRegistration, SHARED_TASK_RESOURCE_HANDLES};
use multiplexed::SHARED_CONNECTION;
use redis::aio::ConnectionLike;
use redis::{Cmd, Pipeline, RedisError, RedisResult};
use tokio::task::yield_now;
use tracing::{debug, error, trace};

mod conlike;
mod multiplexed;
mod owned;

#[allow(clippy::enum_variant_names)]
#[derive(Eq, PartialEq)]
enum CommandParseMode {
    ArgumentCount,
    ArgumentSize,
    Argument,
}

/// Redis connection that monitors for connection errors
pub struct RedisResource<C: ConnectionLike> {
    pub(super) con: C,
    pub(super) handle: HandleRegistration,
    logging_enabled: bool,
}

impl<C: ConnectionLike> RedisResource<C> {
    /// Enables request logging
    pub fn set_logging(&mut self, enabled: bool) {
        self.logging_enabled = enabled;
    }

    /// Set the redis database index
    pub async fn select(&mut self, db: usize) -> RedisResult<()> {
        debug!(db, "Selecting redis database");

        Ok(redis::cmd("SELECT")
            .arg(db)
            .query_async(&mut self.con)
            .await?)
    }

    async fn notify(&mut self, error: &RedisError) {
        error!(?error, "Redis connection encountered error");

        self.handle.resource_died().await;

        if self.handle.is_shared {
            // Invalidate the shared connection
            trace!("Invalidating shared connection");
            *(SHARED_CONNECTION.lock().await) = None;

            // Notify all other task's handles that are using the shared connection
            trace!("Notifying sibling task handles");
            let handles = SHARED_TASK_RESOURCE_HANDLES.lock().await;
            for handle in handles.iter() {
                handle.clone().resource_died().await;
            }
        }

        yield_now().await;
    }

    fn log_cmd(&self, cmd: &Cmd) {
        let packed_command: Vec<u8> = cmd.get_packed_command();
        self.print_packed_command(packed_command);
    }

    fn log_pipeline(&self, pipeline: &Pipeline) {
        let packed_command: Vec<u8> = pipeline.get_packed_pipeline();
        self.print_packed_command(packed_command);
    }

    fn print_packed_command(&self, cmd: Vec<u8>) {
        let input = String::from_utf8_lossy(&cmd);

        let mut chars = input.chars().peekable();
        let mut mode = CommandParseMode::ArgumentCount;
        let mut output = String::new();

        while let Some(char) = chars.next() {
            // Advance to the next line
            if let Some(next_char) = chars.peek() {
                if char == '\r' && *next_char == '\n' {
                    chars.next(); // skip the \n
                    mode = match mode {
                        CommandParseMode::ArgumentCount => CommandParseMode::ArgumentSize,
                        CommandParseMode::ArgumentSize => CommandParseMode::Argument,
                        CommandParseMode::Argument => {
                            output += " ";
                            CommandParseMode::ArgumentSize
                        }
                    };
                    continue;
                }
            }

            // Print argument content
            if mode == CommandParseMode::Argument {
                output.push(char);
            }
        }

        output += "\n";

        trace!(command = ?output, "Redis TX");
    }
}
