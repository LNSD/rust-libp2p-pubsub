use std::future::Future;
use std::pin::Pin;

use rand::Rng;

use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::SwarmBuilder;
use libp2p::Swarm;
use libp2p_pubsub_core::{Behaviour as PubsubBehaviour, Config, IdentTopic};
use libp2p_pubsub_floodsub::Protocol as Floodsub;
use tracing_futures::Instrument;

type Behaviour = PubsubBehaviour<Floodsub>;

use crate::transport;

/// Creates a new IdentTopic with the form "/pubsub/2/it-pubsub-test-{NUM}"
/// where {NUM} is a random u32.
pub fn new_test_topic() -> IdentTopic {
    IdentTopic::new(format!(
        "/pubsub/2/it-pubsub-test-{}",
        rand::thread_rng().gen::<u32>()
    ))
}

/// Creates a new Node with the given key-pair, default Config and default
/// Protocol.
pub fn new_test_node(keypair: &Keypair) -> Swarm<Behaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = transport::test_transport(keypair);
    let config = Config::default();
    let protocol = Default::default();
    let behaviour = Behaviour::new(config.clone(), protocol);
    SwarmBuilder::with_executor(
        transport,
        behaviour,
        peer_id,
        |fut: Pin<Box<dyn Future<Output = ()> + Send>>| {
            tokio::spawn(fut.in_current_span());
        },
    )
    .build()
}
