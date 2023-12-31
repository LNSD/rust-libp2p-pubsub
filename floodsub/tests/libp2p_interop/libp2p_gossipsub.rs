use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use assert_matches::assert_matches;
use futures::StreamExt;
use libp2p::gossipsub::{
    Behaviour as Libp2pGossipsubBehaviour, Config as Libp2pGossipsubConfig,
    ConfigBuilder as Libp2pGossipsubConfigBuilder, Event as Libp2pGossipsubEvent,
    IdentTopic as Libp2pGossipsubIdentTopic,
    MessageAuthenticity as Libp2pGossipsubMessageAuthenticity,
    ValidationMode as Libp2pGossipsubValidationMode,
};
use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::{Swarm, SwarmBuilder, SwarmEvent};
use rand::Rng;
use tokio::time::timeout;
use tracing_futures::Instrument;
use void::Void;

use libp2p_pubsub_core::{Behaviour as PubsubBehaviour, Config, Event, IdentTopic, Message};
use libp2p_pubsub_floodsub::Protocol as Floodsub;
use testlib::any_memory_addr;
use testlib::keys::{TEST_KEYPAIR_A, TEST_KEYPAIR_B};

type Behaviour = PubsubBehaviour<Floodsub>;

/// Create a new test topic with a random name.
fn new_test_topic() -> IdentTopic {
    IdentTopic::new(format!(
        "/pubsub/2/it-pubsub-test-{}",
        rand::thread_rng().gen::<u32>()
    ))
}

/// Create a new test topic from a given string..
fn new_libp2p_topic(raw: &str) -> Libp2pGossipsubIdentTopic {
    Libp2pGossipsubIdentTopic::new(raw)
}

/// Create a new test node with the given keypair and config.
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

/// Create a new libp2p gossipsub test node with the given keypair and config.
fn new_libp2p_gossipsub_node(
    keypair: &Keypair,
    privacy: Libp2pGossipsubMessageAuthenticity,
    config: Libp2pGossipsubConfig,
) -> Swarm<Libp2pGossipsubBehaviour> {
    let peer_id = PeerId::from(keypair.public());
    let transport = testlib::test_transport(keypair);
    let behaviour =
        Libp2pGossipsubBehaviour::new(privacy, config).expect("valid gossipsub configuration");
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

async fn wait_for_message_event(
    swarm: &mut Swarm<Behaviour>,
) -> Vec<SwarmEvent<Event, Infallible>> {
    let mut events = Vec::new();

    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(?event, "Event emitted");
        events.push(event);

        if matches!(
            events.last(),
            Some(SwarmEvent::Behaviour(Event::MessageReceived { .. }))
        ) {
            break;
        }
    }

    events
}

async fn wait_for_libp2p_gossipsub_message_event(
    swarm: &mut Swarm<Libp2pGossipsubBehaviour>,
) -> Vec<SwarmEvent<Libp2pGossipsubEvent, Void>> {
    let mut events = Vec::new();

    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(?event, "Event emitted");
        events.push(event);

        if matches!(
            events.last(),
            Some(SwarmEvent::Behaviour(Libp2pGossipsubEvent::Message { .. }))
        ) {
            break;
        }
    }

    events
}

async fn wait_mesh_message_propagation(
    duration: Duration,
    swarm1: &mut Swarm<Behaviour>,
    swarm2: &mut Swarm<Libp2pGossipsubBehaviour>,
) -> Vec<SwarmEvent<Libp2pGossipsubEvent, Void>> {
    tokio::select! {
        _ = timeout(duration, testlib::swarm::poll(swarm1)) => panic!("timeout reached"),
        res = wait_for_libp2p_gossipsub_message_event(swarm2) => res,
    }
}

async fn wait_mesh_libp2p_gossipsub_message_propagation(
    duration: Duration,
    swarm1: &mut Swarm<Libp2pGossipsubBehaviour>,
    swarm2: &mut Swarm<Behaviour>,
) -> Vec<SwarmEvent<Event, Infallible>> {
    tokio::select! {
        _ = timeout(duration, testlib::swarm::poll(swarm1)) => panic!("timeout reached"),
        res = wait_for_message_event(swarm2) => res,
    }
}

/// Interoperability test where a Floodsub node acts publisher and a Libp2p Gosssipsub Node (with
/// Floodsub support enabled) acts as subscriber.
///
/// The publisher sends a message to the pubsub topic, the subscriber asserts the propagation and
/// reception of the message.
#[tokio::test]
async fn floodsub_node_publish_and_gossipsub_node_subscribes() {
    testlib::init_logger();

    //// Given
    let topic = new_test_topic();
    let libp2p_topic = new_libp2p_topic(topic.hash().as_str());

    let message_payload = b"test-payload";

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    let publisher_config = Config::default();
    let subscriber_config = Libp2pGossipsubConfigBuilder::default()
        .validation_mode(Libp2pGossipsubValidationMode::Permissive)
        .support_floodsub()
        .build()
        .expect("valid gossipsub configuration");

    let mut publisher = new_test_node(&publisher_key, publisher_config.clone());
    testlib::swarm::should_listen_on_address(&mut publisher, any_memory_addr());

    let mut libp2p_subscriber = new_libp2p_gossipsub_node(
        &subscriber_key,
        Libp2pGossipsubMessageAuthenticity::Anonymous,
        subscriber_config.clone(),
    );
    testlib::swarm::should_listen_on_address(&mut libp2p_subscriber, any_memory_addr());

    let (_publisher_addr, subscriber_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut publisher, &mut libp2p_subscriber),
    )
    .await
    .expect("listening to start");

    // Subscribe to the topic
    publisher
        .behaviour_mut()
        .subscribe(topic.clone())
        .expect("subscribe to topic");
    libp2p_subscriber
        .behaviour_mut()
        .subscribe(&libp2p_topic)
        .expect("subscribe to topic");

    // Poll the pub-sub network to process the subscriptions
    testlib::swarm::poll_mesh(
        Duration::from_micros(10),
        &mut publisher,
        &mut libp2p_subscriber,
    )
    .await;

    // Dial the publisher node
    testlib::swarm::should_dial_address(&mut publisher, subscriber_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut publisher, &mut libp2p_subscriber),
    )
    .await
    .expect("publisher to dial the subscriber");

    testlib::swarm::poll_mesh(
        Duration::from_millis(50),
        &mut publisher,
        &mut libp2p_subscriber,
    )
    .await;

    //// When
    let message = Message::new(topic.clone(), *message_payload);
    publisher
        .behaviour_mut()
        .publish(message)
        .expect("publish the message");

    let sub_events = wait_mesh_message_propagation(
        Duration::from_millis(50),
        &mut publisher,
        &mut libp2p_subscriber,
    )
    .await;

    //// Then
    let last_event = sub_events.last().expect("at least one event");
    assert_matches!(last_event, SwarmEvent::Behaviour(Libp2pGossipsubEvent::Message { message, .. }) => {
        assert!(message.sequence_number.is_none());
        assert!(message.source.is_none());
        assert_eq!(message.topic.as_str(), topic.hash().as_str());
        assert_eq!(message.data[..], message_payload[..]);
    });
}

/// Interoperability test where a Libp2p Gossipsub node (with Floodsub support enabled) acts
/// publisher and a Floodsub node acts as subscriber.
///
/// The publisher sends a message to the pubsub topic, the subscriber asserts the propagation and
/// reception of the message.
#[tokio::test]
async fn gossipsub_node_publish_and_floodsub_node_subscribes() {
    testlib::init_logger();

    //// Given
    let topic = new_test_topic();
    let libp2p_topic = new_libp2p_topic(topic.hash().as_str());

    let message_payload = b"test-payload";

    let publisher_key = testlib::secp256k1_keypair(TEST_KEYPAIR_A);
    let subscriber_key = testlib::secp256k1_keypair(TEST_KEYPAIR_B);

    let libp2p_publisher_config = Libp2pGossipsubConfigBuilder::default()
        .validation_mode(Libp2pGossipsubValidationMode::Permissive)
        .support_floodsub()
        .build()
        .expect("valid gossipsub configuration");
    let subscriber_config = Config::default();

    let mut libp2p_publisher = new_libp2p_gossipsub_node(
        &subscriber_key,
        Libp2pGossipsubMessageAuthenticity::Anonymous,
        libp2p_publisher_config.clone(),
    );
    testlib::swarm::should_listen_on_address(&mut libp2p_publisher, any_memory_addr());

    let mut subscriber = new_test_node(&publisher_key, subscriber_config.clone());
    testlib::swarm::should_listen_on_address(&mut subscriber, any_memory_addr());

    let (libp2p_publisher_addr, _subscriber_addr) = timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_start_listening(&mut libp2p_publisher, &mut subscriber),
    )
    .await
    .expect("listening to start");

    // Subscribe to the topic
    libp2p_publisher
        .behaviour_mut()
        .subscribe(&libp2p_topic)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(topic.clone())
        .expect("subscribe to topic");

    // Dial the publisher node
    testlib::swarm::should_dial_address(&mut subscriber, libp2p_publisher_addr);
    timeout(
        Duration::from_secs(5),
        testlib::swarm::wait_for_connection_establishment(&mut subscriber, &mut libp2p_publisher),
    )
    .await
    .expect("publisher to dial the subscriber");

    testlib::swarm::poll_mesh(
        Duration::from_millis(50),
        &mut subscriber,
        &mut libp2p_publisher,
    )
    .await;

    //// When
    libp2p_publisher
        .behaviour_mut()
        .publish(libp2p_topic.hash(), *message_payload)
        .expect("publish the message");

    let sub_events = wait_mesh_libp2p_gossipsub_message_propagation(
        Duration::from_millis(50),
        &mut libp2p_publisher,
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
        assert_eq!(src, libp2p_publisher.local_peer_id(), "The message should be propagated by the publisher");
        // Assert the message
        assert!(message.sequence_number.is_none());
        assert!(message.from.is_none());
        assert_eq!(message.topic.as_str(), topic.hash().as_str());
        assert_eq!(message.data, message_payload[..]);
    });
}
