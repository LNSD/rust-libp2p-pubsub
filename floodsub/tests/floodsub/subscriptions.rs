use std::time::Duration;

use assert_matches::assert_matches;
use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::SwarmBuilder;
use libp2p::Swarm;
use rand::Rng;
use tokio::time::timeout;

use common_test as testlib;
use common_test::any_memory_addr;
use common_test::keys::{TEST_KEYPAIR_A, TEST_KEYPAIR_B};
use floodsub::{Behaviour, Config, IdentTopic};

fn new_test_topic() -> IdentTopic {
    IdentTopic::new(format!(
        "/pubsub/2/it-pubsub-test-{}",
        rand::thread_rng().gen::<u32>()
    ))
}

fn new_test_node(keypair: &Keypair, config: Config) -> Swarm<Behaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = testlib::test_transport(keypair).expect("create the transport");
    let behaviour = Behaviour::new(config);
    SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build()
}

#[tokio::test]
async fn send_subscriptions_on_connection_established() {
    testlib::init_logger();

    //// Given
    let pubsub_topic_a = new_test_topic();
    let pubsub_topic_b = new_test_topic();
    let pubsub_topic_c = new_test_topic();

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

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

    // Subscribe to the topic
    publisher
        .behaviour_mut()
        .subscribe(&pubsub_topic_a)
        .expect("subscribe to topic");
    publisher
        .behaviour_mut()
        .subscribe(&pubsub_topic_b)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic_a)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic_c)
        .expect("subscribe to topic");

    //// When
    // Dial the publisher node
    subscriber.dial(publisher_addr).expect("dial to succeed");
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut subscriber, &mut publisher),
    )
    .await
    .expect("subscriber to connect to publisher");

    // Wait for pub-sub network to establish
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut publisher, &mut subscriber).await;

    //// Then
    let topic_a = pubsub_topic_a.hash();
    let topic_b = pubsub_topic_b.hash();
    let topic_c = pubsub_topic_c.hash();

    let publisher_router = publisher.behaviour().router();
    let subscriber_router = subscriber.behaviour().router();

    let publisher_subscriptions = publisher_router
        .subscriptions()
        .cloned()
        .collect::<Vec<_>>();
    assert!(publisher_subscriptions.contains(&topic_a));
    assert!(publisher_subscriptions.contains(&topic_b));
    assert_eq!(publisher_subscriptions.len(), 2);

    let subscriber_subscriptions = subscriber_router
        .subscriptions()
        .cloned()
        .collect::<Vec<_>>();
    assert!(subscriber_subscriptions.contains(&topic_a));
    assert!(subscriber_subscriptions.contains(&topic_c));
    assert_eq!(subscriber_subscriptions.len(), 2);

    assert_matches!(
        publisher_router.peer_subscriptions(subscriber.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a));
            assert!(subscriptions.contains(&topic_c));
            assert_eq!(subscriptions.len(), 2);
        }
    );

    assert_matches!(
        subscriber_router.peer_subscriptions(publisher.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a));
            assert!(subscriptions.contains(&topic_b));
            assert_eq!(subscriptions.len(), 2);
        }
    );
}

#[tokio::test]
async fn send_subscriptions_on_subscribe() {
    testlib::init_logger();

    //// Given
    let pubsub_topic_a = new_test_topic();
    let pubsub_topic_b = new_test_topic();

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

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

    // Subscribe to the topic
    publisher
        .behaviour_mut()
        .subscribe(&pubsub_topic_a)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic_a)
        .expect("subscribe to topic");

    // Dial the publisher node
    testlib::swarm::should_dial_address(&mut subscriber, publisher_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut subscriber, &mut publisher),
    )
    .await
    .expect("subscriber to connect to publisher");

    //// When
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic_b)
        .expect("subscribe to topic");

    // Wait for pub-sub network to establish
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut publisher, &mut subscriber).await;

    //// Then
    let topic_a = pubsub_topic_a.hash();
    let topic_b = pubsub_topic_b.hash();

    let publisher_router = publisher.behaviour().router();
    let subscriber_router = subscriber.behaviour().router();

    let publisher_subscriptions = publisher_router
        .subscriptions()
        .cloned()
        .collect::<Vec<_>>();
    assert!(publisher_subscriptions.contains(&topic_a));
    assert_eq!(publisher_subscriptions.len(), 1);

    let subscriber_subscriptions = subscriber_router
        .subscriptions()
        .cloned()
        .collect::<Vec<_>>();
    assert!(subscriber_subscriptions.contains(&topic_a));
    assert!(subscriber_subscriptions.contains(&topic_b));
    assert_eq!(subscriber_subscriptions.len(), 2);

    assert_matches!(
        publisher_router.peer_subscriptions(subscriber.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a));
            assert!(subscriptions.contains(&topic_b));
            assert_eq!(subscriptions.len(), 2);
        }
    );

    assert_matches!(
        subscriber_router.peer_subscriptions(publisher.local_peer_id()),
        Some(subscriptions) => {
            assert!(subscriptions.contains(&topic_a));
            assert_eq!(subscriptions.len(), 1);
        }
    );
}

#[tokio::test]
async fn send_subscriptions_on_unsubscribe() {
    testlib::init_logger();

    //// Given
    let pubsub_topic_a = new_test_topic();
    let pubsub_topic_b = new_test_topic();
    let pubsub_topic_c = new_test_topic();

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

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

    // Subscribe to the topic
    publisher
        .behaviour_mut()
        .subscribe(&pubsub_topic_a)
        .expect("subscribe to topic");
    publisher
        .behaviour_mut()
        .subscribe(&pubsub_topic_b)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic_a)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic_c)
        .expect("subscribe to topic");

    // Dial the publisher node
    testlib::swarm::should_dial_address(&mut subscriber, publisher_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut subscriber, &mut publisher),
    )
    .await
    .expect("subscriber to connect to publisher");

    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut publisher, &mut subscriber).await;

    //// When
    publisher
        .behaviour_mut()
        .unsubscribe(&pubsub_topic_a)
        .expect("unsubscribe from topic");
    subscriber
        .behaviour_mut()
        .unsubscribe(&pubsub_topic_a)
        .expect("unsubscribe from topic");

    // Wait for pub-sub network to establish
    testlib::swarm::poll_mesh(Duration::from_millis(10), &mut publisher, &mut subscriber).await;

    //// Then
    let topic_a = pubsub_topic_a.hash();
    let topic_b = pubsub_topic_b.hash();
    let topic_c = pubsub_topic_c.hash();

    let publisher_router = publisher.behaviour().router();
    let subscriber_router = subscriber.behaviour().router();

    let publisher_subscriptions = publisher_router
        .subscriptions()
        .cloned()
        .collect::<Vec<_>>();
    assert!(!publisher_subscriptions.contains(&topic_a));
    assert!(publisher_subscriptions.contains(&topic_b));
    assert_eq!(publisher_subscriptions.len(), 1);

    let subscriber_subscriptions = subscriber_router
        .subscriptions()
        .cloned()
        .collect::<Vec<_>>();
    assert!(!subscriber_subscriptions.contains(&topic_a));
    assert!(subscriber_subscriptions.contains(&topic_c));
    assert_eq!(subscriber_subscriptions.len(), 1);

    assert_matches!(
        publisher_router.peer_subscriptions(subscriber.local_peer_id()),
        Some(subscriptions) => {
            assert!(!subscriptions.contains(&topic_a));
            assert!(subscriptions.contains(&topic_c));
            assert_eq!(subscriptions.len(), 1);
        }
    );

    assert_matches!(
        subscriber_router.peer_subscriptions(publisher.local_peer_id()),
        Some(subscriptions) => {
            assert!(!subscriptions.contains(&topic_a));
            assert!(subscriptions.contains(&topic_b));
            assert_eq!(subscriptions.len(), 1);
        }
    );
}
