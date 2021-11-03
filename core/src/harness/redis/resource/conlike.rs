use super::RedisResource;
use futures::FutureExt;
use redis::aio::ConnectionLike;
use redis::{Cmd, Pipeline, RedisFuture, Value};
use tracing::trace;

/// Handle a redis command result.
macro_rules! notify_if_disconnected {
    ($self:expr, $result:expr) => {
        if let Err(ref e) = $result {
            if e.is_connection_dropped()
                || e.is_io_error()
                || e.is_connection_refusal()
                || e.is_timeout()
            {
                $self.notify(e).await;
            }
        }
    };
}

impl<C: ConnectionLike + Send> ConnectionLike for RedisResource<C> {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        (async move {
            if self.logging_enabled {
                self.log_cmd(cmd);
            }

            let result = self.con.req_packed_command(cmd).await;

            if self.logging_enabled {
                match result {
                    Ok(ref value) => trace!(?value, "Redis RECV"),
                    Err(ref error) => trace!(?error, "Redis RECV failed"),
                }
            }

            notify_if_disconnected!(self, result);
            result
        })
        .boxed()
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        (async move {
            if self.logging_enabled {
                self.log_pipeline(cmd);
            }

            let result = self.con.req_packed_commands(cmd, offset, count).await;

            if self.logging_enabled {
                match &result {
                    Ok(ref values) => {
                        for value in values {
                            trace!(?value, "Redis RECV");
                        }
                    }
                    Err(ref error) => {
                        trace!(?error, "Redis RECV failed");
                    }
                }
            }

            notify_if_disconnected!(self, result);
            result
        })
        .boxed()
    }

    fn get_db(&self) -> i64 {
        self.con.get_db()
    }
}
