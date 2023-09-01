use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use assert_matches::assert_matches;
use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::SwarmBuilder;
use libp2p::Swarm;
use rand::Rng;
use tokio::time::timeout;
use tracing_futures::Instrument;

use common_test as testlib;
use common_test::any_memory_addr;
use common_test::keys::{TEST_KEYPAIR_A, TEST_KEYPAIR_B};
use floodsub::Protocol as Floodsub;
use pubsub::{Behaviour as PubsubBehaviour, Config, IdentTopic};

type Behaviour = PubsubBehaviour<Floodsub>;

fn new_test_topic() -> IdentTopic {
    IdentTopic::new(format!(
        "/pubsub/2/it-pubsub-test-{}",
        rand::thread_rng().gen::<u32>()
    ))
}

fn new_test_node(keypair: &Keypair, config: Config) -> Swarm<Behaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = testlib::test_transport(keypair);
    let behaviour = Behaviour::new(config, Default::default());
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

#[tokio::test]
async fn node_should_subscribe_to_topic() {
    testlib::init_logger();

    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    let node_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);

    let mut node = new_test_node(&node_key, Default::default());

    //// When
    node.behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");
    node.behaviour_mut()
        .subscribe(topic_b.clone())
        .expect("subscribe to topic");

    // Poll the node for a short period of time to allow the subscriptions to be processed.
    testlib::swarm::poll_node(Duration::from_micros(10), &mut node).await;

    //// Then
    let topic_a = topic_a.hash();
    let topic_b = topic_b.hash();

    let subscriptions = node.behaviour().subscriptions();
    assert_eq!(
        subscriptions.len(),
        2,
        "Node should be subscribed to 2 topics"
    );
    assert!(
        subscriptions.contains(&topic_a),
        "Node should be subscribed to Topic A"
    );
    assert!(
        subscriptions.contains(&topic_b),
        "Node should be subscribed to Topic B"
    );
}

#[tokio::test]
async fn node_should_unsubscribe_from_topic() {
    testlib::init_logger();

    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let topic_c = new_test_topic();

    let node_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);

    let mut node = new_test_node(&node_key, Default::default());

    // Subscribe to the test topics
    node.behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");
    node.behaviour_mut()
        .subscribe(topic_b.clone())
        .expect("subscribe to topic");
    node.behaviour_mut()
        .subscribe(topic_c.clone())
        .expect("subscribe to topic");

    // Poll the node for a short period of time to allow the subscriptions to be processed.
    testlib::swarm::poll_node(Duration::from_micros(10), &mut node).await;

    //// When
    node.behaviour_mut()
        .unsubscribe(&topic_b)
        .expect("unsubscribe from topic");

    // Poll the node for a short period of time to allow the subscriptions to be processed.
    testlib::swarm::poll_node(Duration::from_micros(10), &mut node).await;

    //// Then
    let topic_a = topic_a.hash();
    let topic_b = topic_b.hash();
    let topic_c = topic_c.hash();

    let subscriptions = node.behaviour().subscriptions();
    assert_eq!(
        subscriptions.len(),
        2,
        "Node should be subscribed to 2 topics"
    );

    assert!(
        subscriptions.contains(&topic_a),
        "Node should be subscribed to Topic A"
    );
    assert!(
        !subscriptions.contains(&topic_b),
        "Node should not be subscribed to Topic B"
    );
    assert!(
        subscriptions.contains(&topic_c),
        "Node should be subscribed to Topic C"
    );
}

#[tokio::test]
async fn send_subscriptions_on_connection_established() {
    testlib::init_logger();

    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let topic_c = new_test_topic();

    let node_a_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let node_b_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    let mut node_a = new_test_node(&node_a_key, Default::default());
    testlib::swarm::should_listen_on_address(&mut node_a, any_memory_addr());

    let mut node_b = new_test_node(&node_b_key, Default::default());
    testlib::swarm::should_listen_on_address(&mut node_b, any_memory_addr());

    let (node_a_addr, _node_b_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut node_a, &mut node_b),
    )
    .await
    .expect("listening to start");

    // Subscribe to the topic
    node_a
        .behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");
    node_a
        .behaviour_mut()
        .subscribe(topic_b.clone())
        .expect("subscribe to topic");
    node_b
        .behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");
    node_b
        .behaviour_mut()
        .subscribe(topic_c.clone())
        .expect("subscribe to topic");

    //// When
    // Dial the Node A
    node_b.dial(node_a_addr).expect("dial to succeed");
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut node_b, &mut node_a),
    )
    .await
    .expect("Node A to connect to Node B");

    // Wait for pub-sub network to establish
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut node_a, &mut node_b).await;

    //// Then
    let topic_a = topic_a.hash();
    let topic_b = topic_b.hash();
    let topic_c = topic_c.hash();

    assert_matches!(
        node_a.behaviour().peer_subscriptions(node_b.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a), "Node A should be aware of Node B subscription to Topic A");
            assert!(subscriptions.contains(&topic_c), "Node A should be aware of Node B subscription to Topic C");
            assert_eq!(subscriptions.len(), 2);
        },
        "Node A should be aware of Node B's topic subscriptions"
    );

    assert_matches!(
        node_b.behaviour().peer_subscriptions(node_a.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a), "Node B should be aware of Node A subscription to Topic A");
            assert!(subscriptions.contains(&topic_b), "Node B should be aware of Node A subscription to Topic B");
            assert_eq!(subscriptions.len(), 2);
        },
        "Node B should be aware of Node A's topic subscriptions"
    );
}

#[tokio::test]
async fn send_subscriptions_on_subscribe() {
    testlib::init_logger();

    //// Given
    let topic_a = new_test_topic();

    let node_a_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let node_b_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    let mut node_a = new_test_node(&node_a_key, Default::default());
    testlib::swarm::should_listen_on_address(&mut node_a, any_memory_addr());

    let mut node_b = new_test_node(&node_b_key, Default::default());
    testlib::swarm::should_listen_on_address(&mut node_b, any_memory_addr());

    let (node_a_addr, _node_b_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut node_a, &mut node_b),
    )
    .await
    .expect("listening to start");

    // Node B dial Node A
    testlib::swarm::should_dial_address(&mut node_b, node_a_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut node_b, &mut node_a),
    )
    .await
    .expect("Node B to connect to Node A");

    // Poll the network for a short period of time to allow the subscriptions to be processed and exchanged.
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut node_a, &mut node_b).await;

    //// When
    node_b
        .behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");

    // Poll the network for a short period of time to allow the subscriptions to be processed and exchanged.
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut node_a, &mut node_b).await;

    //// Then
    let topic_a = topic_a.hash();

    assert_matches!(
        node_a.behaviour().peer_subscriptions(node_b.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a), "Node A should be aware of Node B subscription to Topic A");
            assert_eq!(subscriptions.len(), 1);
        },
        "Node A should be aware of Node B's topic subscriptions"
    );
}

#[tokio::test]
async fn send_subscriptions_on_unsubscribe() {
    testlib::init_logger();

    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let topic_c = new_test_topic();

    let node_a_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let node_b_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    let mut node_a = new_test_node(&node_a_key, Default::default());
    testlib::swarm::should_listen_on_address(&mut node_a, any_memory_addr());

    let mut node_b = new_test_node(&node_b_key, Default::default());
    testlib::swarm::should_listen_on_address(&mut node_b, any_memory_addr());

    let (node_a_addr, _node_b_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut node_a, &mut node_b),
    )
    .await
    .expect("listening to start");

    // Subscribe to the topic
    node_a
        .behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");
    node_a
        .behaviour_mut()
        .subscribe(topic_b.clone())
        .expect("subscribe to topic");
    node_b
        .behaviour_mut()
        .subscribe(topic_a.clone())
        .expect("subscribe to topic");
    node_b
        .behaviour_mut()
        .subscribe(topic_c.clone())
        .expect("subscribe to topic");

    // Dial the node_a node
    testlib::swarm::should_dial_address(&mut node_b, node_a_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut node_b, &mut node_a),
    )
    .await
    .expect("Node B to connect to Node A");

    // Poll the network for a short period of time to allow the subscriptions to be processed and exchanged.
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut node_a, &mut node_b).await;

    //// When
    node_a
        .behaviour_mut()
        .unsubscribe(&topic_a)
        .expect("unsubscribe from topic");
    node_b
        .behaviour_mut()
        .unsubscribe(&topic_a)
        .expect("unsubscribe from topic");

    // Poll the network for a short period of time to allow the subscriptions to be processed.
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut node_a, &mut node_b).await;

    //// Then
    let topic_a = topic_a.hash();
    let topic_b = topic_b.hash();
    let topic_c = topic_c.hash();

    assert_matches!(
        node_a.behaviour().peer_subscriptions(node_b.local_peer_id()),
        Some(subscriptions) => {
            assert!(!subscriptions.contains(&topic_a), "Node B should not be subscribed to Topic A");
            assert!(subscriptions.contains(&topic_c), "Node B should be subscribed to Topic C");
            assert_eq!(subscriptions.len(), 1);
        },
        "Node A should be aware of Node B's topic subscriptions"
    );

    assert_matches!(
        node_b.behaviour().peer_subscriptions(node_a.local_peer_id()),
        Some(subscriptions) => {
            assert!(!subscriptions.contains(&topic_a), "Node A should not be subscribed to Topic A");
            assert!(subscriptions.contains(&topic_b), "Node A should be subscribed to Topic B");
            assert_eq!(subscriptions.len(), 1);
        },
        "Node B should be aware of Node A's topic subscriptions"
    );
}
