use std::rc::Rc;
use std::time::Duration;

use bytes::Bytes;
use rand::random;
use sha2::{Digest, Sha256};

use common::service::BufferedContext;
use common_test as testlib;

use crate::framing::Message;
use crate::message_id::{MessageId, MessageIdFn};
use crate::topic::TopicHash;

use super::events::{ServiceIn as MessageCacheInEvent, SubscriptionEvent};
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

/// Create a test `Message` with given topic and random payload.
fn new_test_message(topic: TopicHash) -> Message {
    let payload = format!("test-payload-{}", random::<u32>());
    Message::new(topic, payload.into_bytes())
}

/// A custom `MessageIdFn` that returns the message payload as the message ID.
fn custom_message_id(msg: &Message) -> MessageId {
    let mut hasher = Sha256::new();
    hasher.update(msg.topic_str());
    hasher.update(msg.data());
    MessageId::new(hasher.finalize().to_vec())
}

/// Create a topic configuration event sequence.
fn new_subscription_seq(
    topic: TopicHash,
    id_fn: Option<Rc<MessageIdFn>>,
) -> impl IntoIterator<Item = MessageCacheInEvent> {
    [MessageCacheInEvent::SubscriptionEvent(
        SubscriptionEvent::Subscribed {
            topic,
            message_id_fn: id_fn,
        },
    )]
}

/// Create an unsubscription event sequence.
fn new_unsubscription_seq(topic: TopicHash) -> impl IntoIterator<Item = MessageCacheInEvent> {
    [MessageCacheInEvent::SubscriptionEvent(
        SubscriptionEvent::Unsubscribed(topic),
    )]
}

/// Create a message received event sequence.
fn new_message_received_seq(message: Message) -> impl IntoIterator<Item = MessageCacheInEvent> {
    [MessageCacheInEvent::MessageReceived(Rc::new(message))]
}

/// Create a message published event sequence.
fn new_message_published_seq(message: Message) -> impl IntoIterator<Item = MessageCacheInEvent> {
    [MessageCacheInEvent::MessagePublished(Rc::new(message))]
}

#[tokio::test]
async fn unknown_topic_message_topic_not_added_to_cache() {
    //// Given
    let mut service = new_test_service();

    let topic = new_test_topic();
    let message_a = Message::new_with_sequence_number(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
    );
    let message_b = Message::new_with_sequence_number(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
    );

    //// When
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone()),
        new_message_received_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        !service.contains(&message_a),
        "Cache should not contain message A"
    );
    assert!(
        !service.contains(&message_b),
        "Cache should not contain message B"
    );
}

#[tokio::test]
async fn not_seen_message_added_to_cache_with_default_message_id_fn() {
    //// Given
    let mut service = new_test_service();

    let topic = new_test_topic();
    let message_a = Message::new_with_sequence_number(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
    );
    let message_b = Message::new_with_sequence_number(
        topic.clone(),
        b"test-payload".to_vec(),
        new_test_seqno(),
    );

    // Simulate a subscription to the topic
    let input_events = new_subscription_seq(topic.clone(), None);
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone()),
        new_message_received_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        service.contains(&message_a),
        "Cache should contain message A"
    );
    assert!(
        service.contains(&message_b),
        "Cache should contain message B"
    );
}

#[tokio::test]
async fn not_seen_received_message_is_added_to_cache() {
    //// Given
    let mut service = new_test_service();

    let topic_a = new_test_topic();
    let topic_b = new_test_topic();
    let message_a = new_test_message(topic_a.clone());
    let message_b = new_test_message(topic_b.clone());

    // Simulate a subscription to the topics
    let input_events = itertools::chain!(
        new_subscription_seq(topic_a.clone(), Some(Rc::new(custom_message_id))),
        new_subscription_seq(topic_b.clone(), Some(Rc::new(custom_message_id)))
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone()),
        new_message_received_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        service.contains(&message_a),
        "Cache should contain message A"
    );
    assert!(
        service.contains(&message_b),
        "Cache should contain message B"
    );
}

#[tokio::test]
async fn not_seen_published_message_is_added_to_cache() {
    //// Given
    let mut service = new_test_service();

    let topic = new_test_topic();
    let message_a = new_test_message(topic.clone());
    let message_b = new_test_message(topic.clone());

    // Simulate a subscription to the topic
    let input_events = new_subscription_seq(topic.clone(), Some(Rc::new(custom_message_id)));
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    let input_events = itertools::chain!(
        new_message_published_seq(message_a.clone()),
        new_message_published_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        service.contains(&message_a),
        "Cache should contain message A"
    );
    assert!(
        service.contains(&message_b),
        "Cache should contain message B"
    );
}

#[tokio::test]
async fn when_unsubscribed_topic_messages_should_not_be_added_to_cache() {
    //// Given
    let mut service = new_test_service();

    let topic = new_test_topic();
    let message_a = new_test_message(topic.clone());
    let message_b = new_test_message(topic.clone());

    // Simulate a subscription to the topic
    let input_events = new_subscription_seq(topic.clone(), Some(Rc::new(custom_message_id)));
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    // Simulate an unsubscription from the topic
    let input_events = new_unsubscription_seq(topic.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Simulate a message received event
    let input_events = itertools::chain!(
        new_message_received_seq(message_a.clone()),
        new_message_received_seq(message_b.clone())
    );
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(
        !service.contains(&message_a),
        "Cache should not contain message A"
    );
    assert!(
        !service.contains(&message_b),
        "Cache should not contain message B"
    );
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

    // Simulate a subscription to the topic
    let input_events = new_subscription_seq(topic.clone(), Some(Rc::new(custom_message_id)));
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    // Simulate a message received event
    let input_events = new_message_received_seq(message.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(60)).await;

    //// Then
    assert!(
        !service.contains(&message),
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

    // Simulate a subscription to the topic
    let input_events = new_subscription_seq(topic.clone(), Some(Rc::new(custom_message_id)));
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    // Simulate a message received event
    let input_events = new_message_received_seq(message.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Simulate a message received event
    // This should update the cached message timestamp
    let input_events = new_message_received_seq(message.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// Then
    assert!(service.contains(&message), "Cache should contain message");
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

    // Simulate a subscription to the topic
    let input_events = new_subscription_seq(topic.clone(), Some(Rc::new(custom_message_id)));
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    //// When
    // Simulate a message received event
    let input_events = new_message_received_seq(message.clone());
    testlib::service::inject_events(&mut service, input_events);
    testlib::service::async_poll(&mut service).await;

    // Wait for heartbeat interval to expire
    // THis should clear all expired messages from the cache
    tokio::time::sleep(Duration::from_millis(60)).await;

    //// Then
    assert!(
        !service.contains(&message),
        "Cache should not contain message"
    );
}
