use std::time::Duration;

use assert_matches::assert_matches;
use libp2p::swarm::SwarmEvent;
use libp2p::Swarm;
use tokio::time::timeout;

use libp2p_pubsub_core::{Behaviour as PubsubBehaviour, Event, Hasher, Message, Topic};
use libp2p_pubsub_floodsub::Protocol as Floodsub;
use testlib::any_memory_addr;
use testlib::keys::{TEST_KEYPAIR_A, TEST_KEYPAIR_B};

type Behaviour = PubsubBehaviour<Floodsub>;

use crate::floodsub::flood_testlib::*;

/// Subscribe to a topic and assert that the subscription is successful.
#[tracing::instrument(skip_all, fields(swarm = % swarm.local_peer_id()))]
fn should_subscribe_to_topic<H: Hasher>(swarm: &mut Swarm<Behaviour>, topic: Topic<H>) -> bool {
    let result = swarm.behaviour_mut().subscribe(topic);

    assert_matches!(result, Ok(_), "subscribe to topic should succeed");

    result.unwrap()
}

/// Publish to a topic and assert that the publish is successful.
///
/// Returns the `MessageId` of the published message.
#[tracing::instrument(skip_all, fields(swarm = % swarm.local_peer_id()))]
fn should_publish_to_topic(swarm: &mut Swarm<Behaviour>, message: Message) {
    let result = swarm.behaviour_mut().publish(message);

    assert_matches!(result, Ok(_), "publish to topic should succeed");
}

#[tokio::test]
async fn publish_to_topic() {
    testlib::init_logger();

    //// Given
    let topic = new_test_topic();
    let message_payload = b"test-payload";

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    //// Setup
    let mut publisher = new_test_node(&publisher_key);
    testlib::swarm::should_listen_on_address(&mut publisher, any_memory_addr());

    let mut subscriber = new_test_node(&subscriber_key);
    testlib::swarm::should_listen_on_address(&mut subscriber, any_memory_addr());

    let (publisher_addr, _subscriber_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut publisher, &mut subscriber),
    )
    .await
    .expect("listening to start");

    // Subscribe to the topic
    should_subscribe_to_topic(&mut publisher, topic.clone());
    should_subscribe_to_topic(&mut subscriber, topic.clone());

    // Poll the pub-sub network to process the subscriptions
    testlib::swarm::poll_mesh(Duration::from_micros(10), &mut publisher, &mut subscriber).await;

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
    let message = Message::new(topic.clone(), *message_payload);
    should_publish_to_topic(&mut publisher, message);

    let (_, sub_events) = testlib::swarm::poll_mesh_and_collect_events(
        Duration::from_millis(50),
        &mut publisher,
        &mut subscriber,
    )
    .await;

    //// Then
    assert_eq!(
        sub_events.len(),
        1,
        "Only 1 message event should be emitted"
    );
    assert_matches!(&sub_events[0], SwarmEvent::Behaviour(Event::MessageReceived { src, message, .. }) => {
        // Assert the propagation peer
        assert_eq!(src, publisher.local_peer_id(), "The message should be propagated by the publisher");
        // Assert the message
        assert!(message.sequence_number.is_none());
        assert!(message.from.is_none());
        assert_eq!(message.topic.as_str(), topic.hash().as_str());
        assert_eq!(message.data, message_payload[..]);
    });
}
