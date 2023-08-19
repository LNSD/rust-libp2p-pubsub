use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use assert_matches::assert_matches;
use bytes::Bytes;
use futures::StreamExt;
use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::Swarm;
use rand::Rng;
use tokio::time::timeout;
use tracing_futures::Instrument;

use common_test as testlib;
use common_test::any_memory_addr;
use common_test::keys::{TEST_KEYPAIR_A, TEST_KEYPAIR_B};
use floodsub::{Behaviour, Config, Event, Hasher, IdentTopic, Topic};

/// Create a new test topic with a random name.
fn new_test_topic() -> IdentTopic {
    IdentTopic::new(format!(
        "/pubsub/2/it-pubsub-test-{}",
        rand::thread_rng().gen::<u32>()
    ))
}

/// Create a new test node with the given keypair and config.
fn new_test_node(keypair: &Keypair, config: Config) -> Swarm<Behaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = testlib::test_transport(keypair);
    let behaviour = Behaviour::new(config);
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

/// Subscribe to a topic and assert that the subscription is successful.
#[tracing::instrument(skip_all, fields(swarm = % swarm.local_peer_id()))]
fn should_subscribe_to_topic<H: Hasher>(swarm: &mut Swarm<Behaviour>, topic: &Topic<H>) -> bool {
    let result = swarm.behaviour_mut().subscribe(topic);

    assert_matches!(result, Ok(_), "subscribe to topic should succeed");

    result.unwrap()
}

/// Publish to a topic and assert that the publish is successful.
///
/// Returns the `MessageId` of the published message.
#[tracing::instrument(skip_all, fields(swarm = % swarm.local_peer_id()))]
fn should_publish_to_topic<H: Hasher>(
    swarm: &mut Swarm<Behaviour>,
    topic: &Topic<H>,
    data: impl Into<Vec<u8>>,
) {
    let result = swarm.behaviour_mut().publish(topic, data);

    assert_matches!(result, Ok(_), "publish to topic should succeed");
}

/// Wait for a message event to be received by the swarm.
///
/// Returns all the events received by the swarm until the message event is received.
async fn wait_for_message(swarm: &mut Swarm<Behaviour>) -> Vec<SwarmEvent<Event, Infallible>> {
    let mut events = Vec::new();

    loop {
        let event = swarm.select_next_some().await;
        events.push(event);

        if matches!(
            events.last(),
            Some(SwarmEvent::Behaviour(Event::Message { .. }))
        ) {
            break;
        }
    }

    events
}

#[tokio::test]
async fn publish_to_topic() {
    testlib::init_logger();

    //// Given
    let pubsub_topic = new_test_topic();
    let message_payload = Bytes::from_static(b"test-payload");

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    let pubsub_config = Config::default();

    //// Setup
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
    should_subscribe_to_topic(&mut publisher, &pubsub_topic);
    should_subscribe_to_topic(&mut subscriber, &pubsub_topic);

    // Dial the publisher node
    testlib::swarm::should_dial_address(&mut subscriber, publisher_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut subscriber, &mut publisher),
    )
    .await
    .expect("subscriber to connect to publisher");

    // Wait for pub-sub network to establish
    testlib::swarm::poll_mesh(Duration::from_millis(50), &mut publisher, &mut subscriber).await;

    //// When
    should_publish_to_topic(&mut publisher, &pubsub_topic, message_payload.clone());

    let (_, sub_events) = testlib::swarm::poll_mesh_and_collect_events(
        Duration::from_millis(50),
        &mut publisher,
        &mut subscriber,
    )
    .await;

    //// Then
    let messages = sub_events
        .into_iter()
        .filter(|ev| matches!(ev, SwarmEvent::Behaviour(Event::Message { .. })))
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 1);

    let last_message = messages.last().expect("at least one event");
    assert_matches!(last_message, SwarmEvent::Behaviour(Event::Message { message, .. }) => {
        assert!(message.sequence_number().is_some());
        assert!(message.source().is_none());
        assert_eq!(message.topic_str(), pubsub_topic.hash().as_str());
        assert_eq!(message.data()[..], message_payload[..]);
    });
}
