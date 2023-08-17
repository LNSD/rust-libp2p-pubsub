use std::time::Duration;

use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::{Swarm, SwarmBuilder};
use tokio::time::timeout;

use common_test as testlib;
use floodsub::{Behaviour, Config};
use testlib::any_memory_addr;

fn new_test_node(keypair: &Keypair, config: Config) -> Swarm<Behaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = testlib::test_transport(keypair).expect("create the transport");
    let behaviour = Behaviour::new(config);
    SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build()
}

#[tokio::test]
async fn connection_is_established() {
    testlib::init_logger();

    //// Given
    let publisher_key = testlib::secp256k1_keypair(testlib::keys::TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(testlib::keys::TEST_KEYPAIR_B);

    let pubsub_config = Config::default();

    let mut publisher = new_test_node(&publisher_key, pubsub_config.clone());
    testlib::swarm::should_listen_on_address(&mut publisher, any_memory_addr());

    let mut subscriber = new_test_node(&subscriber_key, pubsub_config.clone());
    testlib::swarm::should_listen_on_address(&mut subscriber, any_memory_addr());

    let (publisher_addr, _subscriber_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut publisher, &mut subscriber),
    )
    .await
    .expect("listening to start");

    //// When
    // Dial the publisher node
    testlib::swarm::should_dial_address(&mut subscriber, publisher_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut subscriber, &mut publisher),
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
