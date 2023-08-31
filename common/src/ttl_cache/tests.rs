use std::thread::sleep;
use std::time::Duration;

use assert_matches::assert_matches;
use sha2::{Digest, Sha256};

use super::cache::Cache;

// Test message type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Message {
    pub topic: String,
    pub payload: Vec<u8>,
}

// Test message id type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MessageId(Vec<u8>);

/// A helper function to compute the message id of a message.
fn message_id(msg: &Message) -> MessageId {
    let mut hasher = Sha256::new();
    hasher.update(&msg.payload);
    MessageId(hasher.finalize().to_vec())
}

/// A helper function to create a test message.
fn test_message(payload: &'static [u8]) -> (MessageId, Message) {
    let message = Message {
        topic: String::from("test-topic"),
        payload: payload.to_vec(),
    };
    let id = message_id(&message);
    (id, message)
}

#[test]
fn insert_and_check_if_contained() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");

    let mut cache = Cache::default();

    //// When
    let result1 = cache.put(id1.clone(), msg1.clone());
    let result2 = cache.put(id2.clone(), msg2.clone());
    let result3 = cache.put(id3.clone(), msg3.clone());

    // Insert a message with the same id as message 1
    let result4 = cache.put(id1.clone(), msg1.clone());

    //// Then
    // Assert insertion results
    assert!(result1, "message 1 should have been inserted");
    assert!(result2, "message 2 should have been inserted");
    assert!(result3, "message 3 should have been inserted");

    // Asset insertion result of message that was already in the cache
    assert!(!result4, "message 1 should not have been inserted again");

    // Assert cache contents
    assert_eq!(cache.len(), 3, "cache should contain 3 messages");

    assert!(cache.contains_key(&id1), "message 1 should be in the cache");
    assert!(cache.contains_key(&id2), "message 2 should be in the cache");
    assert!(cache.contains_key(&id3), "message 3 should be in the cache");
}

#[test]
fn iterate_over_cache_content() {
    //// iGiven
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");

    let cache = {
        let mut cache = Cache::default();

        cache.put(id1.clone(), msg1.clone());
        cache.put(id2.clone(), msg2.clone());
        cache.put(id3.clone(), msg3.clone());

        cache
    };

    //// When
    let mut cache_iter = cache.iter();

    //// Then
    // Assert cache content and order
    assert_matches!(cache_iter.next(), Some((id, msg)) => {
        assert_eq!(id, &id1, "message 1 id should be correct");
        assert_eq!(msg, &msg1, "message 1 should be correct");
    });

    assert_matches!(cache_iter.next(), Some((id, msg)) => {
        assert_eq!(id, &id2, "message 2 id should be correct");
        assert_eq!(msg, &msg2, "message 2 should be correct");
    });

    assert_matches!(cache_iter.next(), Some((id, msg)) => {
        assert_eq!(id, &id3, "message 3 id should be correct");
        assert_eq!(msg, &msg3, "message 3 should be correct");
    });

    assert!(
        cache_iter.next().is_none(),
        "no more entries should be present"
    );
}

#[test]
fn insert_over_capacity() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");
    let (id4, msg4) = test_message(b"test-message4");

    // Limit cache capacity to 2 messages
    let capacity = 2;
    let ttl = Duration::from_secs(5);
    let mut cache = Cache::with_capacity_and_ttl(capacity, ttl);

    //// When
    cache.put(id1.clone(), msg1);
    cache.put(id2.clone(), msg2);

    // Insert messages to go over capacity and discard the previous
    cache.put(id3.clone(), msg3);
    cache.put(id4.clone(), msg4);

    //// Then
    assert_eq!(cache.len(), 2, "cache should contain 2 messages");

    let cache_content_ids = cache.iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(cache_content_ids, vec![&id3, &id4]);
}

#[test]
fn check_if_message_is_contained() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");
    let (id4, msg4) = test_message(b"test-message4");
    let (id5, msg5) = test_message(b"test-message5");

    // Set cache TTL to 100ms
    let capacity = 1024;
    let ttl = Duration::from_millis(100);
    let mut cache = Cache::with_capacity_and_ttl(capacity, ttl);

    cache.put(id1.clone(), msg1.clone());
    cache.put(id2.clone(), msg2.clone());
    cache.put(id3.clone(), msg3.clone());

    // Insert messages 120ms apart (so the ones inserted first expire)
    sleep(ttl + Duration::from_millis(20));

    cache.put(id4.clone(), msg4.clone());
    cache.put(id5.clone(), msg5.clone());

    //// When
    let valid_entry = cache.contains_key(&id4);
    let expired_entry = cache.contains_key(&id2);

    //// Then
    assert_eq!(cache.len(), 2, "cache should contain 2 messages");

    assert!(valid_entry, "message 4 should be in the cache");
    assert!(!expired_entry, "message 2 should not be in the cache");
}

#[test]
fn get_a_cache_entry_by_id() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");
    let (id4, msg4) = test_message(b"test-message4");
    let (id5, msg5) = test_message(b"test-message5");

    // Set cache TTL to 100ms
    let capacity = 1024;
    let ttl = Duration::from_millis(100);
    let mut cache = Cache::with_capacity_and_ttl(capacity, ttl);

    cache.put(id1.clone(), msg1.clone());
    cache.put(id2.clone(), msg2.clone());
    cache.put(id3.clone(), msg3.clone());

    // Insert messages 120ms apart (so the ones inserted first expire)
    sleep(ttl + Duration::from_millis(20));

    cache.put(id4.clone(), msg4.clone());
    cache.put(id5.clone(), msg5.clone());

    //// When
    let valid_entry = cache.get(&id4);
    let expired_entry = cache.get(&id2);

    //// Then
    assert_eq!(cache.len(), 2, "cache should contain 2 messages");

    assert_matches!(valid_entry, Some(msg) => {
        assert_eq!(msg, &msg4, "message 4 should be correct");
    });

    assert_matches!(expired_entry, None);
}

#[test]
fn remove_a_cache_entry_by_id() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");
    let (id4, msg4) = test_message(b"test-message4");
    let (id5, msg5) = test_message(b"test-message5");

    // Set cache TTL to 100ms
    let capacity = 1024;
    let ttl = Duration::from_millis(100);
    let mut cache = Cache::with_capacity_and_ttl(capacity, ttl);

    cache.put(id1.clone(), msg1.clone());
    cache.put(id2.clone(), msg2.clone());
    cache.put(id3.clone(), msg3.clone());

    // Insert messages 120ms apart (so the ones inserted first expire)
    sleep(ttl + Duration::from_millis(20));

    cache.put(id4.clone(), msg4.clone());
    cache.put(id5.clone(), msg5.clone());

    //// When
    let valid_entry = cache.remove(&id4);
    let expired_entry = cache.remove(&id2);

    //// Then
    assert_eq!(cache.len(), 1, "cache should contain 2 messages");

    assert_matches!(valid_entry, Some(msg) => {
        assert_eq!(msg, msg4, "message 4 should be correct");
    });

    assert_matches!(expired_entry, None);
}

#[test]
fn remove_all_expired_entries_at_once() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");
    let (id4, msg4) = test_message(b"test-message4");
    let (id5, msg5) = test_message(b"test-message5");

    // Set cache TTL to 100ms
    let capacity = 1024;
    let ttl = Duration::from_millis(100);
    let mut cache = Cache::with_capacity_and_ttl(capacity, ttl);

    //// When
    cache.put(id1.clone(), msg1);
    cache.put(id2.clone(), msg2);
    cache.put(id3.clone(), msg3);

    // Insert messages 120ms apart
    sleep(ttl + Duration::from_millis(20));

    cache.put(id4.clone(), msg4);
    cache.put(id5.clone(), msg5);

    // Remove expired entries
    cache.clear_expired_entries();

    //// Then
    // Assert cache contents
    assert_eq!(cache.len(), 2, "cache should contain 2 messages");

    let cache_content_ids = cache.iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(cache_content_ids, vec![&id4, &id5]);
}

/// When inserting a message that is already in the cache, the timestamp should be updated
/// and the cache entry moved to the back of the cache entries list.
#[test]
fn insert_an_already_expired_message_should_update_the_timestamp() {
    //// Given
    let (id1, msg1) = test_message(b"test-message1");
    let (id2, msg2) = test_message(b"test-message2");
    let (id3, msg3) = test_message(b"test-message3");
    let (id4, msg4) = test_message(b"test-message4");
    let (id5, msg5) = test_message(b"test-message5");

    // Set cache TTL to 100ms
    let capacity = 1024;
    let ttl = Duration::from_millis(100);
    let mut cache = Cache::with_capacity_and_ttl(capacity, ttl);

    //// When
    cache.put(id1.clone(), msg1.clone());
    cache.put(id2.clone(), msg2.clone());
    cache.put(id3.clone(), msg3.clone());

    // Insert messages 120ms apart
    sleep(ttl + Duration::from_millis(20));

    cache.put(id4.clone(), msg4);
    cache.put(id5.clone(), msg5);

    // Insert again an already expired message
    let update_result = cache.put(id2.clone(), msg2);

    //// Then
    // Assert insertion results
    assert!(update_result, "message 2 should have been inserted again");

    // Assert cache contents: 2 expired (1 and 3) + 3 valid (2, 4 and 5)
    let cache_content_ids = cache.iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(
        cache_content_ids,
        vec![
            &id1, &id3, // expired
            &id4, &id5, &id2, // valid
        ]
    );
}
