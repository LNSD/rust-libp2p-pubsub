use std::convert::Infallible;
use std::time::Duration;

use assert_matches::assert_matches;
use bytes::Bytes;
use futures::StreamExt;
use libp2p::identity::{Keypair, PeerId};
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{Multiaddr, Swarm};
use rand::Rng;
use tokio::time::timeout;

use floodsub::{Behaviour, Config, Event, IdentTopic};

use crate::testlib;
use crate::testlib::any_memory_addr;
use crate::testlib::keys::{TEST_KEYPAIR_A, TEST_KEYPAIR_B};

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

async fn poll_mesh(
    duration: Duration,
    swarm1: &mut Swarm<Behaviour>,
    swarm2: &mut Swarm<Behaviour>,
) {
    timeout(
        duration,
        futures::future::join(testlib::swarm::poll(swarm1), testlib::swarm::poll(swarm2)),
    )
    .await
    .expect_err("timeout to be reached");
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

async fn wait_mesh_message_propagation(
    duration: Duration,
    swarm1: &mut Swarm<Behaviour>,
    swarm2: &mut Swarm<Behaviour>,
) -> Vec<SwarmEvent<Event, Infallible>> {
    tokio::select! {
        _ = timeout(duration, testlib::swarm::poll(swarm1)) => panic!("timeout reached"),
        res = wait_for_message(swarm2) => res,
    }
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

    // Subscribe to the topic
    publisher
        .behaviour_mut()
        .subscribe(&pubsub_topic)
        .expect("subscribe to topic");
    subscriber
        .behaviour_mut()
        .subscribe(&pubsub_topic)
        .expect("subscribe to topic");

    // Dial the publisher node
    subscriber.dial(publisher_addr).expect("dial to succeed");
    timeout(
        Duration::from_secs(5),
        wait_for_connection_establishment(&mut subscriber, &mut publisher),
    )
    .await
    .expect("subscriber to connect to publisher");

    // Wait for pub-sub network to establish
    poll_mesh(Duration::from_millis(50), &mut publisher, &mut subscriber).await;

    //// When
    publisher
        .behaviour_mut()
        .publish(&pubsub_topic, message_payload.clone())
        .expect("publish the message");

    let sub_events =
        wait_mesh_message_propagation(Duration::from_millis(50), &mut publisher, &mut subscriber)
            .await;

    //// Then
    let last_event = sub_events.last().expect("at least one event");
    assert_matches!(last_event, SwarmEvent::Behaviour(Event::Message { message, .. }) => {
        assert!(message.sequence_number().is_some());
        assert!(message.source().is_none());
        assert_eq!(message.topic_str(), pubsub_topic.hash().as_str());
        assert_eq!(message.data()[..], message_payload[..]);
    });
}
