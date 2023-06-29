use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// The maximum size of a RPC frame.
    pub max_frame_size: usize,

    /// The idle timeout of a connection.
    pub connection_idle_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_frame_size: 65537,
            connection_idle_timeout: Duration::from_secs(120),
        }
    }
}
