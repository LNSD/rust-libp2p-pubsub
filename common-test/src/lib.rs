pub use keys::secp256k1_keypair;
pub use transport::*;

pub mod keys;
pub mod swarm;
pub mod transport;

/// Initialize the logger for tests.
pub fn init_logger() {
    let _ = pretty_env_logger::formatted_builder()
        .is_test(true)
        .try_init();
}
