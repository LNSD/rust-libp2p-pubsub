use std::rc::Rc;

use assert_matches::assert_matches;
use libp2p::identity::PeerId;
use rand::random;

use libp2p_pubsub_core::protocol::{
    ProtocolRouterConnectionEvent, ProtocolRouterInEvent, ProtocolRouterMessageEvent,
    ProtocolRouterOutEvent, ProtocolRouterSubscriptionEvent,
};
use libp2p_pubsub_core::{FrameMessage, MessageId, TopicHash};
use testlib::service::noop_context;

use super::Router;

/// Create a new random test topic.
fn new_test_topic() -> TopicHash {
    TopicHash::from_raw(format!("/pubsub/2/it-pubsub-test-{}", random::<u32>()))
}

/// Create a new random peer ID.
fn new_test_peer_id() -> PeerId {
    PeerId::random()
}

/// Creates a new random 256 bits message id.
fn new_test_message_id() -> MessageId {
    MessageId::new(random::<[u8; 32]>())
}

/// Create a random message with the given topic.
fn new_test_message(topic: TopicHash) -> FrameMessage {
    FrameMessage::new(topic, b"test-payload".to_vec())
}

/// Create a new message received sequence for the given topic.
fn new_received_message_seq(
    src: PeerId,
    topic: TopicHash,
) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::MessageEvent(
        ProtocolRouterMessageEvent::MessageReceived {
            src,
            message: Rc::new(new_test_message(topic)),
            message_id: new_test_message_id(),
        },
    )]
}

/// Create a new message published sequence for the given topic.
fn new_published_message_seq(topic: TopicHash) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::MessageEvent(
        ProtocolRouterMessageEvent::MessagePublished {
            message: Rc::new(new_test_message(topic)),
            message_id: new_test_message_id(),
        },
    )]
}

/// Create a new subscription sequence for the given topic.
fn new_subscribe_seq(topic: TopicHash) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::SubscriptionEvent(
        ProtocolRouterSubscriptionEvent::Subscribed(topic.into()),
    )]
}

/// Create a new unsubscription sequence for the given topic.
fn new_unsubscribe_seq(topic: TopicHash) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::SubscriptionEvent(
        ProtocolRouterSubscriptionEvent::Unsubscribed(topic),
    )]
}

/// Create a new peer disconnection event sequence for the given peer.
fn new_peer_disconnected_seq(peer: PeerId) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::ConnectionEvent(
        ProtocolRouterConnectionEvent::PeerDisconnected(peer),
    )]
}

/// Create a new peer subscription sequence for the given peer and topic.
fn new_peer_subscribed_seq(
    peer: PeerId,
    topic: TopicHash,
) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::SubscriptionEvent(
        ProtocolRouterSubscriptionEvent::PeerSubscribed { peer, topic },
    )]
}

/// Create a new peer unsubscription sequence for the given peer and topic.
fn new_peer_unsubscribed_seq(
    peer: PeerId,
    topic: TopicHash,
) -> impl IntoIterator<Item = ProtocolRouterInEvent> {
    [ProtocolRouterInEvent::SubscriptionEvent(
        ProtocolRouterSubscriptionEvent::PeerUnsubscribed { peer, topic },
    )]
}

#[test]
fn do_not_forward_a_message_if_not_subscribed() {
    //// Given
    let topic_a = new_test_topic(); // Subscribed
    let topic_b = new_test_topic(); // Not subscribed

    let remote_peer = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the subscription to Topic A and the remote peer subscription to topics A and B
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_peer_subscribed_seq(remote_peer, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer, topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the reception of a message from the remote peer on topic B
    let input_events = new_received_message_seq(remote_peer, topic_b.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert!(output_events.is_empty(), "No message should be forwarded");
}

#[test]
fn do_not_forward_a_message_if_no_peers_are_subscribed() {
    //// Given
    let topic = new_test_topic();
    let remote_peer = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the subscription to topic
    let input_events = new_subscribe_seq(topic.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the reception of a message from the remote peer on topic B
    let input_events = new_received_message_seq(remote_peer, topic.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert!(output_events.is_empty(), "No message should be forwarded");
}

#[test]
fn do_not_forward_a_message_if_sender_is_the_only_peer_subscribed() {
    //// Given
    let topic = new_test_topic();
    let remote_peer = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the subscription to topic
    let input_events = itertools::chain!(
        new_subscribe_seq(topic.clone()),
        new_peer_subscribed_seq(remote_peer, topic.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the reception of a message from the remote peer on topic B
    let input_events = new_received_message_seq(remote_peer, topic.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert!(output_events.is_empty(), "No message should be forwarded");
}

#[test]
fn do_not_forward_a_published_message_if_no_peers_are_subscribed() {
    //// Given
    let topic = new_test_topic();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the subscription to topic
    let input_events = new_subscribe_seq(topic.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the publication of a message on the topic
    let input_events = new_published_message_seq(topic.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert!(output_events.is_empty(), "No message should be forwarded");
}

#[test]
fn do_not_forward_messages_after_unsubscribing_a_topic() {
    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let remote_peer = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the subscription to topic
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone()),
        new_peer_subscribed_seq(remote_peer, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer, topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        // Simulate the unsubscription from Topic A
        new_unsubscribe_seq(topic_a.clone()),
        // Simulate the publication of two messages, one on Topic A and one Topic B
        new_published_message_seq(topic_a.clone()),
        new_published_message_seq(topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(
        output_events.len(),
        1,
        "Only one message should be forwarded"
    );
    assert_matches!(&output_events[0], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 1, "The message should be forwarded to 1 peer");
        assert!(dest.contains(&remote_peer), "The message should be forwarded to peer");
        // Assert message
        assert_eq!(&message.topic(), &topic_b, "The message should be on topic B");
    });
}

#[test]
fn publish_a_message_to_all_peers_subscribed() {
    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let remote_peer_a = new_test_peer_id();
    let remote_peer_b = new_test_peer_id();
    let remote_peer_c = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the local node and peers subscriptions
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic_b.clone()),
        new_peer_subscribed_seq(remote_peer_c, topic_a.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the reception of a message from the remote peer on topic B
    let input_events = new_published_message_seq(topic_a.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(
        output_events.len(),
        1,
        "A message should be forwarded to all peers subscribed"
    );
    assert_matches!(&output_events[0], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 2, "The message should be forwarded to 2 peers");
        assert!(dest.contains(&remote_peer_a), "The message should be forwarded to peer A");
        assert!(dest.contains(&remote_peer_c), "The message should be forwarded to peer C");
        // Assert message
        assert_eq!(&message.topic(), &topic_a, "The message should be on topic A");
    });
}

#[test]
fn forward_a_message_to_all_peers_subscribed_except_the_sender() {
    //// Given
    let topic = new_test_topic();
    let remote_peer_a = new_test_peer_id();
    let remote_peer_b = new_test_peer_id();
    let remote_peer_c = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the local node and peers subscriptions
    let input_events = itertools::chain!(
        new_subscribe_seq(topic.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic.clone()),
        new_peer_subscribed_seq(remote_peer_c, topic.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the reception of a message from the remote peer B on topic
    let input_events = new_received_message_seq(remote_peer_b, topic.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(
        output_events.len(),
        1,
        "A message forward event should be emitted"
    );
    assert_matches!(&output_events[0], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 2, "The message should be forwarded to 2 peers");
        assert!(dest.contains(&remote_peer_a), "The message should be forwarded to peer A");
        assert!(!dest.contains(&remote_peer_b), "The message should not be forwarded to peer B");
        assert!(dest.contains(&remote_peer_c), "The message should be forwarded to peer C");
        // Assert message
        assert_eq!(&message.topic(), &topic, "The message should be on topic");
    });
}

#[test]
fn topic_should_be_removed_from_routing_table_if_no_remaining_peers() {
    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let remote_peer_a = new_test_peer_id();
    let remote_peer_b = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the local node and peers subscriptions
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic_b.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        // Simulate both remote peers unsubscription from Topic A
        new_peer_unsubscribed_seq(remote_peer_a, topic_a.clone()),
        new_peer_unsubscribed_seq(remote_peer_b, topic_a.clone()),
        // Simulate the publication of a message on Topic A and Topic B
        new_published_message_seq(topic_a.clone()),
        new_published_message_seq(topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(
        output_events.len(),
        1,
        "Only one message should be forwarded"
    );
    assert_matches!(&output_events[0], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 2, "The message should be forwarded to 2 peers");
        assert!(dest.contains(&remote_peer_a), "The message should be forwarded to peer A");
        assert!(dest.contains(&remote_peer_b), "The message should be forwarded to peer B");
        // Assert message
        assert_eq!(&message.topic(), &topic_b, "The message should be on topic B");
    });
}

#[test]
fn peer_should_be_removed_from_routing_table_on_unsubscription_received() {
    //// Given
    let topic = new_test_topic();
    let remote_peer_a = new_test_peer_id();
    let remote_peer_b = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the local node and peers subscriptions
    let input_events = itertools::chain!(
        new_subscribe_seq(topic.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        // Simulate the reception of a peer unsubscription event
        new_peer_unsubscribed_seq(remote_peer_a, topic.clone()),
        // Simulate the publication of a message on the topic
        new_published_message_seq(topic.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(
        output_events.len(),
        1,
        "A message forward event should be emitted"
    );
    assert_matches!(&output_events[0], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 1, "The message should be forwarded to 1 peer");
        assert!(!dest.contains(&remote_peer_a), "The message should not be forwarded to peer A");
        assert!(dest.contains(&remote_peer_b), "The message should be forwarded to peer B");
        // Assert message
        assert_eq!(&message.topic(), &topic, "The message should be on topic");
    });
}

#[test]
fn peer_should_be_removed_from_routing_table_on_disconnect() {
    //// Given
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let remote_peer_a = new_test_peer_id();
    let remote_peer_b = new_test_peer_id();

    let mut service = testlib::service::default_test_service::<Router>();

    // Simulate the local node and peers subscriptions
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer_a, topic_b.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic_a.clone()),
        new_peer_subscribed_seq(remote_peer_b, topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        // Simulate the disconnect of a peer
        new_peer_disconnected_seq(remote_peer_a),
        // Simulate the publication of a message on the topic
        new_published_message_seq(topic_a.clone()),
        new_published_message_seq(topic_b.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(
        output_events.len(),
        2,
        "Two message forward event should be emitted"
    );
    assert_matches!(&output_events[0], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 1, "The message should be forwarded to 1 peer");
        assert!(!dest.contains(&remote_peer_a), "The message should not be forwarded to peer A");
        assert!(dest.contains(&remote_peer_b), "The message should be forwarded to peer B");
        // Assert message
        assert_eq!(&message.topic(), &topic_a, "The message should be on topic");
    });
    assert_matches!(&output_events[1], ProtocolRouterOutEvent::ForwardMessage { dest, message } => {
        // Assert dest nodes
        assert_eq!(dest.len(), 1, "The message should be forwarded to 1 peer");
        assert!(!dest.contains(&remote_peer_a), "The message should not be forwarded to peer A");
        assert!(dest.contains(&remote_peer_b), "The message should be forwarded to peer B");
        // Assert message
        assert_eq!(&message.topic(), &topic_b, "The message should be on topic");
    });
}
