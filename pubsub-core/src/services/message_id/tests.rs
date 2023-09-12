use std::rc::Rc;

use assert_matches::assert_matches;
use bytes::Bytes;
use libp2p::PeerId;
use rand::random;
use sha2::{Digest, Sha256};

use libp2p_pubsub_common::service::BufferedContext;
use testlib::service::noop_context;

use crate::framing::Message;
use crate::message_id::{default_message_id_fn, MessageId, MessageIdFn, MessageRef};
use crate::services::message_id::events::ServiceOut;
use crate::topic::TopicHash;

use super::events::{MessageEvent, ServiceIn as MessageIdInEvent, SubscriptionEvent};
use super::service::MessageIdService;

// Create a test instance of the service under test.
fn new_test_service() -> BufferedContext<MessageIdService> {
    Default::default()
}

/// Create a new test peer id.
fn new_test_peer_id() -> PeerId {
    PeerId::random()
}

/// Create a new random test topic.
fn new_test_topic() -> TopicHash {
    TopicHash::from_raw(format!("/pubsub/2/it-pubsub-test-{}", random::<u32>()))
}

/// Create a new random test sequence number.
fn new_test_seqno() -> Bytes {
    Bytes::from(random::<u32>().to_be_bytes().to_vec())
}

/// A custom `MessageIdFn` that returns the message payload as the message ID.
fn custom_message_id_fn(_src: Option<&PeerId>, msg: &MessageRef) -> MessageId {
    let mut hasher = Sha256::new();
    hasher.update(msg.topic.as_str());
    hasher.update(msg.data.as_ref());
    MessageId::new(hasher.finalize().to_vec())
}

/// Create a topic configuration event sequence.
fn new_subscription_seq(
    topic: TopicHash,
    id_fn: Option<Rc<dyn MessageIdFn<Output = MessageId>>>,
) -> impl IntoIterator<Item = MessageIdInEvent> {
    [MessageIdInEvent::SubscriptionEvent(
        SubscriptionEvent::Subscribed {
            topic,
            message_id_fn: id_fn,
        },
    )]
}

/// Create an unsubscription event sequence.
fn new_unsubscription_seq(topic: TopicHash) -> impl IntoIterator<Item = MessageIdInEvent> {
    [MessageIdInEvent::SubscriptionEvent(
        SubscriptionEvent::Unsubscribed(topic),
    )]
}

/// Create a message received event sequence.
///
/// The propagation source is set to a random peer id.
fn new_message_received_seq(message: Message) -> impl IntoIterator<Item = MessageIdInEvent> {
    [MessageIdInEvent::MessageEvent(MessageEvent::Received {
        src: PeerId::random(),
        message: Rc::new(message),
    })]
}

/// Create a message published event sequence.
fn new_message_published_seq(message: Message) -> impl IntoIterator<Item = MessageIdInEvent> {
    [MessageIdInEvent::MessageEvent(MessageEvent::Published(
        Rc::new(message),
    ))]
}

/// If the node is not subscribed to a topic, the message ID should be generated using the default
/// message ID function.
#[test]
fn unknown_topic_message_uses_default_message_id_fn() {
    //// Given
    let mut service = new_test_service();

    let remote_peer = new_test_peer_id();
    let topic = new_test_topic();

    let message_a = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );
    let message_b = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );

    //// When
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone()),
        new_message_published_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(output_events.len(), 2, "Only 2 events expected");
    assert_matches!(&output_events[0], ServiceOut::MessageReceived { message_id, .. } => {
        let expected_message_id = default_message_id_fn(None, &message_a.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using default message ID function");
    });
    assert_matches!(&output_events[1], ServiceOut::MessagePublished { message_id, .. } => {
        let expected_message_id = default_message_id_fn(None, &message_b.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using default message ID function");
    });
}

/// When the node subscribes to a topic but does not provide a custom message ID function, the
/// default message ID function should be used.
#[test]
fn subscribed_topic_message_uses_default_message_id_fn() {
    //// Given
    let mut service = new_test_service();

    let remote_peer = new_test_peer_id();
    let topic = new_test_topic();

    let message_a = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );
    let message_b = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );

    //// When
    let input_events = itertools::chain!(
        new_subscription_seq(topic.clone(), None),
        new_message_received_seq(message_a.clone()),
        new_message_published_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(output_events.len(), 2, "Only 2 events expected");
    assert_matches!(&output_events[0], ServiceOut::MessageReceived { message_id, .. } => {
        let expected_message_id = default_message_id_fn(None, &message_a.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using default message ID function");
    });
    assert_matches!(&output_events[1], ServiceOut::MessagePublished { message_id, .. } => {
        let expected_message_id = default_message_id_fn(None, &message_b.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using default message ID function");
    });
}

//// When the node subscribes to a topic and provides a custom message ID function, the custom
//// message ID function should be used.
#[test]
fn subscribed_topic_message_uses_configured_custom_message_id_fn() {
    //// Given
    let mut service = new_test_service();

    let remote_peer = new_test_peer_id();
    let topic = new_test_topic();

    let message_a = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );
    let message_b = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );

    let message_id_fn = Rc::new(custom_message_id_fn);

    //// When
    let input_events = itertools::chain!(
        new_subscription_seq(topic.clone(), Some(message_id_fn.clone())),
        new_message_received_seq(message_a.clone()),
        new_message_published_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(output_events.len(), 2, "Only 2 events expected");
    assert_matches!(&output_events[0], ServiceOut::MessageReceived { message_id, .. } => {
        let expected_message_id = message_id_fn(None, &message_a.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using custom message ID function");
    });
    assert_matches!(&output_events[1], ServiceOut::MessagePublished { message_id, .. } => {
        let expected_message_id = message_id_fn(None, &message_b.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using custom message ID function");
    });
}

/// When the node subscribes to a topic and provides a custom message ID function, the custom
/// message ID function should be used. After the node unsubscribes from the topic, the default
/// message ID function should be used.
#[test]
fn unsubscribed_topic_message_uses_default_message_id_fn() {
    //// Given
    let mut service = new_test_service();

    let remote_peer = new_test_peer_id();
    let topic = new_test_topic();

    let message_a = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );
    let message_b = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );
    let message_c = Message::new_with_seq_no_and_from(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
        remote_peer,
    );

    let message_id_fn = Rc::new(custom_message_id_fn);

    //// When
    let input_events = itertools::chain!(
        new_subscription_seq(topic.clone(), Some(message_id_fn.clone())),
        new_message_received_seq(message_a.clone()),
        new_unsubscription_seq(topic.clone()), // Unsubscribe from the topic
        new_message_received_seq(message_b.clone()),
        new_message_published_seq(message_c.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    assert_eq!(output_events.len(), 3, "Only 3 events expected");
    // Assert messages before unsubscription.
    assert_matches!(&output_events[0], ServiceOut::MessageReceived { message_id, .. } => {
        let expected_message_id = message_id_fn(None, &message_a.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using custom message ID function");
    });
    // Assert messages after unsubscription.
    assert_matches!(&output_events[1], ServiceOut::MessageReceived { message_id, .. } => {
        let expected_message_id = default_message_id_fn(None, &message_b.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using default message ID function");
    });
    assert_matches!(&output_events[2], ServiceOut::MessagePublished { message_id, .. } => {
        let expected_message_id = default_message_id_fn(None, &message_c.as_ref().into());
        assert_eq!(message_id, &expected_message_id, "Message ID should have been generated using default message ID function");
    });
}
