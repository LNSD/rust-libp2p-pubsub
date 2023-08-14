use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// The maximum size of a RPC frame.
    max_frame_size: usize,

    /// The idle timeout of a connection.
    connection_idle_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_frame_size: 65537,
            connection_idle_timeout: Duration::from_secs(120),
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
    /// Default is 120 seconds.
    pub fn connection_idle_timeout(&self) -> Duration {
        self.connection_idle_timeout
    }
}
