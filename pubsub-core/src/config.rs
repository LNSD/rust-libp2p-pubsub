use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// The maximum size of a RPC frame.
    max_frame_size: usize,

    /// The idle timeout of a connection.
    connection_idle_timeout: Duration,

    /// The number of retries that will be attempted to send a frame over a connection before
    /// giving up on the connection.
    max_connection_send_retry_attempts: usize,

    /// Time between each heartbeat.
    heartbeat_interval: Duration,

    /// Message cache capacity.
    message_cache_capacity: usize,

    /// Message cache entries Time-To-Live.
    message_cache_ttl: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_frame_size: 65537,
            connection_idle_timeout: Duration::from_secs(120),
            max_connection_send_retry_attempts: 2,
            heartbeat_interval: Duration::from_secs(1),
            message_cache_capacity: 1024,
            message_cache_ttl: Duration::from_secs(5),
        }
    }
}

impl Config {
    /// The maximum byte size for each pubsub frame (default is 65536 bytes).
    ///
    /// This represents the maximum size of the entire protobuf payload. It must be at least
    /// large enough to support basic control messages.
    ///
    /// Default is 65536 bytes.
    pub fn max_frame_size(&self) -> usize {
        self.max_frame_size
    }

    /// The time a connection is maintained to a peer without being in the mesh and without
    /// send/receiving a message from. Connections that idle beyond this timeout are disconnected.
    ///
    /// Default is 120 seconds.
    pub fn connection_idle_timeout(&self) -> Duration {
        self.connection_idle_timeout
    }

    /// The number of retries that will be attempted to send a frame over a connection before
    /// giving up on the connection.
    ///
    /// Default is 2.
    pub fn max_connection_send_retry_attempts(&self) -> usize {
        self.max_connection_send_retry_attempts
    }

    /// The time between each heartbeat.
    ///
    /// Default is 1 second.
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// The maximum number of messages to cache.
    ///
    /// Default is 1024.
    pub fn message_cache_capacity(&self) -> usize {
        self.message_cache_capacity
    }

    /// The time a message is kept in the cache.
    ///
    /// Default is 5 seconds.
    pub fn message_cache_ttl(&self) -> Duration {
        self.message_cache_ttl
    }
}
