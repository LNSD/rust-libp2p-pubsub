use std::time::Duration;

use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::{Swarm, SwarmBuilder};
use libp2p::Multiaddr;
use tokio::time::timeout;

use floodsub::{Behaviour, Config};

use crate::testlib;
use crate::testlib::any_memory_addr;

fn new_test_node(keypair: &Keypair, config: Config) -> Swarm<Behaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = testlib::test_transport(keypair).expect("create the transport");
    let behaviour = Behaviour::new(config);
    SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build()
}

async fn wait_for_start_listening(
    publisher: &mut Swarm<Behaviour>,
    subscriber: &mut Swarm<Behaviour>,
) -> (Multiaddr, Multiaddr) {
    tokio::join!(
        testlib::swarm::wait_for_new_listen_addr(publisher),
        testlib::swarm::wait_for_new_listen_addr(subscriber)
    )
}

async fn wait_for_connection_establishment(
    dialer: &mut Swarm<Behaviour>,
    receiver: &mut Swarm<Behaviour>,
) {
    tokio::join!(
        testlib::swarm::wait_for_connection_established(dialer),
        testlib::swarm::wait_for_connection_established(receiver)
    );
}

#[tokio::test]
async fn connection_is_established() {
    testlib::init_logger();

    //// Given
    let publisher_key = testlib::secp256k1_keypair(testlib::keys::TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(testlib::keys::TEST_KEYPAIR_B);

    let pubsub_config = Config::default();

    let mut publisher = new_test_node(&publisher_key, pubsub_config.clone());
    publisher
        .listen_on(any_memory_addr())
        .expect("listen on address");

    let mut subscriber = new_test_node(&subscriber_key, pubsub_config.clone());
    subscriber
        .listen_on(any_memory_addr())
        .expect("listen on address");

    let (publisher_addr, _subscriber_addr) = timeout(
        Duration::from_secs(5),
        wait_for_start_listening(&mut publisher, &mut subscriber),
    )
    .await
    .expect("listening to start");

    //// When
    // Dial the publisher node
    subscriber.dial(publisher_addr).expect("dial to succeed");
    timeout(
        Duration::from_secs(5),
        wait_for_connection_establishment(&mut subscriber, &mut publisher),
    )
    .await
    .expect("subscriber to connect to publisher");

    //// Then
    let pub_connections = publisher.behaviour().connections();
    let sub_connections = subscriber.behaviour().connections();

    assert_eq!(pub_connections.active_peers_count(), 1);
    assert_eq!(pub_connections.active_peers().len(), 1);
    assert!(pub_connections
        .active_peers()
        .contains(subscriber.local_peer_id()));

    assert_eq!(sub_connections.active_peers_count(), 1);
    assert_eq!(sub_connections.active_peers().len(), 1);
    assert!(sub_connections
        .active_peers()
        .contains(publisher.local_peer_id()));
}
