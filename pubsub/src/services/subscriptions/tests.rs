use assert_matches::assert_matches;
use libp2p::identity::PeerId;
use rand::Rng;

use common_test as testlib;
use common_test::service::noop_context;

use crate::framing::SubscriptionAction;
use crate::services::subscriptions::{
    PeerConnectionEvent, SubscriptionsInEvent, SubscriptionsOutEvent, SubscriptionsService,
};
use crate::topic::{Hasher, IdentityHash, Topic};

/// Create a new random test topic.
fn new_test_topic() -> Topic<IdentityHash> {
    Topic::new(format!(
        "/pubsub/2/it-pubsub-test-{}",
        rand::thread_rng().gen::<u32>()
    ))
}

/// Create a new random peer ID.
fn new_test_peer_id() -> PeerId {
    PeerId::random()
}

/// Create a new subscription sequence for the given topic.
fn new_subscribe_seq<H: Hasher>(topic: Topic<H>) -> impl IntoIterator<Item = SubscriptionsInEvent> {
    [SubscriptionsInEvent::SubscriptionRequest(topic.into())]
}

/// Create a new unsubscription sequence for the given topic.
fn new_unsubscribe_seq<H: Hasher>(
    topic: Topic<H>,
) -> impl IntoIterator<Item = SubscriptionsInEvent> {
    [SubscriptionsInEvent::UnsubscriptionRequest(topic.hash())]
}

/// Create a new peer connection event sequence for the given peer.
fn new_peer_connected_seq(peer: PeerId) -> impl IntoIterator<Item = SubscriptionsInEvent> {
    [SubscriptionsInEvent::PeerConnectionEvent(
        PeerConnectionEvent::NewPeerConnected(peer),
    )]
}

/// Create a new peer disconnection event sequence for the given peer.
fn new_peer_disconnected_seq(peer: PeerId) -> impl IntoIterator<Item = SubscriptionsInEvent> {
    [SubscriptionsInEvent::PeerConnectionEvent(
        PeerConnectionEvent::PeerDisconnected(peer),
    )]
}

/// Create a new peer subscription sequence for the given topic.
fn new_peer_subscribe_seq<H: Hasher>(
    peer: PeerId,
    topic: Topic<H>,
) -> impl IntoIterator<Item = SubscriptionsInEvent> {
    [SubscriptionsInEvent::PeerSubscriptionRequest {
        src: peer,
        action: SubscriptionAction::Subscribe(topic.hash()),
    }]
}

/// Create a new peer unsubscription sequence for the given topic.
fn new_peer_unsubscribe_seq<H: Hasher>(
    peer: PeerId,
    topic: Topic<H>,
) -> impl IntoIterator<Item = SubscriptionsInEvent> {
    [SubscriptionsInEvent::PeerSubscriptionRequest {
        src: peer,
        action: SubscriptionAction::Unsubscribe(topic.hash()),
    }]
}

#[test]
fn register_non_existing_topic_subscription() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let topic_a = new_test_topic();

    //// When
    let input_events = new_subscribe_seq(topic_a.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert!(
        service.is_subscribed(&topic_a.hash()),
        "Node should be subscribed to topic"
    );
    assert!(
        service.subscriptions().contains(&topic_a.hash()),
        "Node should be subscribed to topic"
    );

    // Assert events
    assert_eq!(output_events.len(), 1, "Only 1 event should be emitted");
    assert_matches!(&output_events[0], SubscriptionsOutEvent::Subscribed(sub) => {
        assert_eq!(&sub.topic, &topic_a.hash());
    });
}

#[test]
fn register_existing_topic_subscription() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let topic_a = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = new_subscribe_seq(topic_a.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = new_subscribe_seq(topic_a.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert!(
        service.is_subscribed(&topic_a.hash()),
        "Node should be subscribed to topic"
    );
    assert!(
        service.subscriptions().contains(&topic_a.hash()),
        "Node should be subscribed to topic"
    );

    // Assert events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}

#[test]
fn unregister_non_existing_topic_subscription() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    // Simulate a previous subscription to Topic A
    let input_events = new_subscribe_seq(topic_a.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = new_unsubscribe_seq(topic_b.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert!(
        service.subscriptions().contains(&topic_a.hash()),
        "Node should be subscribed to topic A"
    );
    assert!(
        !service.subscriptions().contains(&topic_b.hash()),
        "Node should not be subscribed to topic B"
    );

    // Assert events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}

#[test]
fn unregister_existing_topic_subscription() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = new_unsubscribe_seq(topic_a.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert!(
        !service.subscriptions().contains(&topic_a.hash()),
        "Node should not be subscribed to Topic A"
    );
    assert!(
        service.subscriptions().contains(&topic_b.hash()),
        "Node should be subscribed to Topic B"
    );

    // Assert events
    assert_eq!(output_events.len(), 1, "Only 1 event should be emitted");
    assert_matches!(&output_events[0], SubscriptionsOutEvent::Unsubscribed(topic) => {
        assert_eq!(topic, &topic_a.hash());
    });
}

#[test]
fn emit_send_subscriptions_on_new_peer_connected() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let peer_a = new_test_peer_id();
    let peer_b = new_test_peer_id();

    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        new_peer_connected_seq(peer_a),
        new_peer_connected_seq(peer_b)
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert events
    assert_eq!(
        output_events.len(),
        2,
        "Only 2 events should be emitted (1 per peer)"
    );
    assert_matches!(&output_events[0], SubscriptionsOutEvent::SendSubscriptions { peer, topics } => {
        assert_eq!(peer, &peer_a);
        assert_eq!(topics.len(), 2);
        assert!(topics.contains(&topic_a.hash()));
        assert!(topics.contains(&topic_b.hash()));
    });
    assert_matches!(&output_events[1], SubscriptionsOutEvent::SendSubscriptions { peer, topics } => {
        assert_eq!(peer, &peer_b);
        assert_eq!(topics.len(), 2);
        assert!(topics.contains(&topic_a.hash()));
        assert!(topics.contains(&topic_b.hash()));
    });
}

#[test]
fn dont_emit_send_subscriptions_on_new_peer_connected_if_no_subscriptions() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let peer_a = new_test_peer_id();
    let peer_b = new_test_peer_id();

    //// When
    let input_events = itertools::chain!(
        new_peer_connected_seq(peer_a),
        new_peer_connected_seq(peer_b)
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}

#[test]
fn register_peer_subscription_to_topic_when_not_registered() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let remote_peer = new_test_peer_id();
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    //// When
    let input_events = itertools::chain!(
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
        new_peer_subscribe_seq(remote_peer, topic_b.clone()),
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
        new_peer_subscribe_seq(remote_peer, topic_b.clone()),
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert the state
    assert!(
        service
            .is_peer_subscribed(&remote_peer, &topic_a.hash())
            .unwrap(),
        "Peer should be subscribed to Topic A"
    );
    assert!(
        service
            .is_peer_subscribed(&remote_peer, &topic_b.hash())
            .unwrap(),
        "Peer should be subscribed to Topic B"
    );

    // Assert the events
    assert_eq!(
        output_events.len(),
        2,
        "Only 2 events should be emitted (1 per topic)"
    );
    assert_matches!(&output_events[0], SubscriptionsOutEvent::PeerSubscribed { peer, topic } => {
        assert_eq!(peer, &remote_peer);
        assert_eq!(topic, &topic_a.hash());
    });
    assert_matches!(&output_events[1], SubscriptionsOutEvent::PeerSubscribed { peer, topic } => {
        assert_eq!(peer, &remote_peer);
        assert_eq!(topic, &topic_b.hash());
    });
}

#[test]
fn register_peer_subscription_to_topic_when_already_registered() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let remote_peer = new_test_peer_id();
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = itertools::chain!(
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
        new_peer_subscribe_seq(remote_peer, topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
        new_peer_subscribe_seq(remote_peer, topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert the events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}

#[test]
fn unregister_peer_subscription_to_topic_when_not_registered() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let remote_peer = new_test_peer_id();
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let topic_c = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = itertools::chain!(
        new_subscribe_seq(topic_a.clone()),
        new_subscribe_seq(topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = new_peer_unsubscribe_seq(remote_peer, topic_c.clone());
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert the events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}

#[test]
fn unregister_peer_subscription_to_topic_when_registered() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let remote_peer = new_test_peer_id();
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = itertools::chain!(
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
        new_peer_subscribe_seq(remote_peer, topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = itertools::chain!(
        new_peer_unsubscribe_seq(remote_peer, topic_a.clone()),
        new_peer_unsubscribe_seq(remote_peer, topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert the state
    assert!(
        !service
            .is_peer_subscribed(&remote_peer, &topic_a.hash())
            .unwrap(),
        "Peer should not be subscribed to Topic A"
    );
    assert!(
        !service
            .is_peer_subscribed(&remote_peer, &topic_b.hash())
            .unwrap(),
        "Peer should not be subscribed to Topic B"
    );

    // Assert the events
    assert_eq!(
        output_events.len(),
        2,
        "Only 2 events should be emitted (1 per topic)"
    );
    assert_matches!(&output_events[0], SubscriptionsOutEvent::PeerUnsubscribed { peer, topic } => {
        assert_eq!(peer, &remote_peer);
        assert_eq!(topic, &topic_a.hash());
    });
    assert_matches!(&output_events[1], SubscriptionsOutEvent::PeerUnsubscribed { peer, topic } => {
        assert_eq!(peer, &remote_peer);
        assert_eq!(topic, &topic_b.hash());
    });
}

#[test]
fn remove_all_peer_subscriptions_on_peer_disconnected() {
    //// Given
    let mut service = testlib::service::default_test_service::<SubscriptionsService>();

    let remote_peer = new_test_peer_id();
    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    // Simulate a previous subscription to the topic
    let input_events = itertools::chain!(
        new_peer_subscribe_seq(remote_peer, topic_a.clone()),
        new_peer_subscribe_seq(remote_peer, topic_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    let input_events = new_peer_disconnected_seq(remote_peer);
    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert!(
        service
            .is_peer_subscribed(&remote_peer, &topic_a.hash())
            .is_none(),
        "Peer should not be subscribed to Topic A"
    );
    assert!(
        service
            .is_peer_subscribed(&remote_peer, &topic_b.hash())
            .is_none(),
        "Peer should not be subscribed to Topic B"
    );

    // Assert the events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}
