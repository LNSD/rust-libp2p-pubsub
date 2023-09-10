use tracing_subscriber::fmt::TestWriter;
use tracing_subscriber::EnvFilter;

pub use keys::secp256k1_keypair;
pub use transport::*;

pub mod keys;
pub mod service;
pub mod swarm;
pub mod test_factory;
pub mod transport;

/// Initialize the logger for tests.
pub fn init_logger() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .with_writer(TestWriter::default())
        .try_init();
}
