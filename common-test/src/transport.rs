#![allow(dead_code)]

use std::time::Duration;

use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::{Boxed, MemoryTransport, Transport};
use libp2p::core::upgrade::Version;
use libp2p::identity::{Keypair, PeerId};
use libp2p::plaintext::PlainText2Config;
use libp2p::{yamux, Multiaddr};

/// Type alias for libp2p transport
pub type P2PTransport = (PeerId, StreamMuxerBox);
/// Type alias for boxed libp2p transport
pub type BoxedP2PTransport = Boxed<P2PTransport>;

/// Any memory address (for testing)
pub fn any_memory_addr() -> Multiaddr {
    "/memory/0".parse().unwrap()
}

/// In memory transport
pub fn test_transport(keypair: &Keypair) -> BoxedP2PTransport {
    let transport = MemoryTransport::default();

    transport
        .upgrade(Version::V1)
        .authenticate(PlainText2Config {
            local_public_key: keypair.public(),
        })
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed()
}
