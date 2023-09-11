use std::rc::Rc;
use std::time::Duration;

use bytes::Bytes;
use libp2p::PeerId;
use rand::random;
use sha2::{Digest, Sha256};

use libp2p_pubsub_common::service::BufferedContext;

use crate::framing::Message;
use crate::message_id::MessageId;
use crate::topic::TopicHash;

use super::events::{MessageEvent, ServiceIn as MessageCacheInEvent};
use super::service::MessageCacheService;

// Create a test instance of the `MessageCacheService`.
fn new_test_service() -> BufferedContext<MessageCacheService> {
    BufferedContext::new(MessageCacheService::new(
        1024,
        Duration::from_secs(5),
        Duration::from_secs(1),
        Duration::from_secs(1),
    ))
}

/// Create a test instance of the `MessageCacheService` with a custom TTL and heartbeat interval.
fn new_test_service_with_ttl_and_heartbeat(
    ttl: Duration,
    heartbeat_interval: Duration,
) -> BufferedContext<MessageCacheService> {
    BufferedContext::new(MessageCacheService::new(
        1024,
        ttl,
        heartbeat_interval,
        Duration::from_secs(0),
    ))
}

/// Create a new random test topic.
fn new_test_topic() -> TopicHash {
    TopicHash::from_raw(format!("/pubsub/2/it-pubsub-test-{}", random::<u32>()))
}

/// Create a new random test sequence number.
fn new_test_seqno() -> Bytes {
    Bytes::from(random::<u32>().to_be_bytes().to_vec())
}

/// Create a new random 256 bits test message ID.
fn new_test_message_id() -> MessageId {
    MessageId::new(random::<[u8; 32]>().to_vec())
}

/// Create a test `Message` with given topic and random payload.
fn new_test_message(topic: TopicHash) -> Message {
    let payload = format!("test-payload-{}", random::<u32>());
    Message::new(topic, payload.into_bytes())
}

/// A custom `MessageIdFn` that returns the message payload as the message ID.
fn custom_message_id_fn(msg: &Message) -> MessageId {
    let mut hasher = Sha256::new();
    hasher.update(msg.topic_str());
    hasher.update(msg.data());
    MessageId::new(hasher.finalize().to_vec())
}

/// Create a message received event sequence.
///
/// The propagation node source is set to a random peer ID.
fn new_message_received_seq(
    message: Message,
    message_id: MessageId,
) -> impl IntoIterator<Item = MessageCacheInEvent> {
    [MessageCacheInEvent::MessageEvent(
        MessageEvent::MessageReceived {
            src: PeerId::random(),
            message: Rc::new(message),
            message_id,
        },
    )]
}

/// Create a message published event sequence.
fn new_message_published_seq(
    message: Message,
    message_id: MessageId,
) -> impl IntoIterator<Item = MessageCacheInEvent> {
    [MessageCacheInEvent::MessageEvent(
        MessageEvent::MessagePublished {
            message: Rc::new(message),
            message_id,
        },
    )]
}

#[tokio::test]
async fn not_seen_message_is_added_to_cache() {
    //// Given
    let mut service = new_test_service();

    let topic_a = new_test_topic();
    let topic_b = new_test_topic();

    let message_a = Message::new_with_sequence_number(
        topic_a.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
    );
    let message_b = Message::new_with_sequence_number(
        topic_b.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
    );

    let message_a_id = custom_message_id_fn(&message_a);
    let message_b_id = custom_message_id_fn(&message_b);
    let unknown_message_id = new_test_message_id();

    //// When
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone(), message_a_id.clone()),
        new_message_published_seq(message_b.clone(), message_b_id.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        service.contains(&message_a_id),
        "Cache should not contain message A"
    );
    assert!(
        service.contains(&message_b_id),
        "Cache should not contain message B"
    );
    assert!(
        !service.contains(&unknown_message_id),
        "Cache should not contain an unknown message id"
    );
}

#[tokio::test]
async fn seen_message_should_not_be_added_to_cache() {
    //// Given
    let mut service = new_test_service();

    let topic = new_test_topic();
    let message_a = new_test_message(topic.clone());
    let message_a_id = custom_message_id_fn(&message_a);
    let message_b = new_test_message(topic.clone());
    let message_b_id = custom_message_id_fn(&message_b);

    //// When
    // Simulate the messages reception
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone(), message_a_id.clone()),
        new_message_published_seq(message_a.clone(), message_a_id.clone()),
        new_message_received_seq(message_b.clone(), message_b_id.clone()),
        new_message_received_seq(message_a.clone(), message_a_id.clone()),
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        service.contains(&message_a_id),
        "Cache should contain message A"
    );
    assert!(
        service.contains(&message_b_id),
        "Cache should contain message B"
    );
    assert_eq!(service.usage(), 2, "Cache should contain 2 messages");
}

/// Test that a message is contained in the cache after TTL expires. THe heartbeat interval is
/// longer than the TTL duration.
#[tokio::test]
async fn seen_message_should_not_be_contained_after_ttl() {
    //// Given
    let mut service =
        new_test_service_with_ttl_and_heartbeat(Duration::from_millis(50), Duration::from_secs(1));

    let topic = new_test_topic();
    let message = new_test_message(topic.clone());
    let message_id = custom_message_id_fn(&message);

    //// When
    // Simulate a message received event
    let input_events = new_message_received_seq(message.clone(), message_id.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(60)).await;

    //// Then
    assert!(
        !service.contains(&message_id),
        "Cache should not contain message"
    );
}

/// Test that a message is contained in the cache after TTL expires and after a new message is
/// received. The heartbeat interval is longer than the TTL duration.
#[tokio::test]
async fn seen_message_timestamp_is_updated() {
    //// Given
    let mut service =
        new_test_service_with_ttl_and_heartbeat(Duration::from_millis(50), Duration::from_secs(1));

    let topic = new_test_topic();
    let message = new_test_message(topic.clone());
    let message_id = custom_message_id_fn(&message);

    //// When
    // Simulate a message received event
    let input_events = new_message_received_seq(message.clone(), message_id.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Simulate a message received event
    // This should update the cached message timestamp
    let input_events = new_message_received_seq(message.clone(), message_id.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        service.contains(&message_id),
        "Cache should contain message"
    );
}

/// Test that a message is not contained in the cache after the heartbeat interval. The cache
/// TTL duration is very close to the heartbeat interval.
#[tokio::test]
async fn seen_message_should_not_be_contained_after_heartbeat() {
    //// Given
    let mut service = new_test_service_with_ttl_and_heartbeat(
        Duration::from_millis(40),
        Duration::from_millis(50),
    );

    let topic = new_test_topic();
    let message = new_test_message(topic.clone());
    let message_id = custom_message_id_fn(&message);

    //// When
    // Simulate a message received event
    let input_events = new_message_received_seq(message.clone(), custom_message_id_fn(&message));
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Wait for heartbeat interval to expire
    // THis should clear all expired messages from the cache
    tokio::time::sleep(Duration::from_millis(60)).await;

    //// Then
    assert!(
        !service.contains(&message_id),
        "Cache should not contain message"
    );
}
