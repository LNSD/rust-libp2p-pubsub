use std::time::Duration;
use testlib::any_memory_addr;
use tokio::time::timeout;

use crate::floodsub::flood_testlib::*;

#[tokio::test]
async fn connection_is_established() {
    testlib::init_logger();

    //// Given
    let node_a_key = testlib::secp256k1_keypair(testlib::keys::TEST_KEYPAIR_A);
    let node_b_key = testlib::secp256k1_keypair(testlib::keys::TEST_KEYPAIR_B);

    let mut node_a = new_test_node(&node_a_key);
    testlib::swarm::should_listen_on_address(&mut node_a, any_memory_addr());

    let mut node_b = new_test_node(&node_b_key);
    testlib::swarm::should_listen_on_address(&mut node_b, any_memory_addr());

    let (node_a_addr, _node_b_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut node_a, &mut node_b),
    )
    .await
    .expect("listening to start");

    //// When
    // Node B dials Node A address.
    testlib::swarm::should_dial_address(&mut node_b, node_a_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut node_b, &mut node_a),
    )
    .await
    .expect("node_b to connect to node_a");

    // Poll the swarm to make sure the connection is established.
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut node_a, &mut node_b).await;

    //// Then
    let node_a_connections = node_a.behaviour().connections();
    let node_b_connections = node_b.behaviour().connections();

    assert_eq!(node_a_connections.active_peers_count(), 1);
    assert_eq!(node_a_connections.active_peers().len(), 1);
    assert!(node_a_connections
        .active_peers()
        .contains(node_b.local_peer_id()));

    assert_eq!(node_b_connections.active_peers_count(), 1);
    assert_eq!(node_b_connections.active_peers().len(), 1);
    assert!(node_b_connections
        .active_peers()
        .contains(node_a.local_peer_id()));
}
