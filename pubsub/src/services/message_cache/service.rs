use std::collections::HashMap;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::StreamExt;

use common::heartbeat::Heartbeat;
use common::service::Service;
use common::ttl_cache::Cache;

use crate::framing::Message;
use crate::message_id::{default_message_id_fn, MessageId, MessageIdFn};
use crate::services::message_cache::MessageCacheInEvent;
use crate::topic::TopicHash;

pub struct MessageCacheService {
    /// The internal cache data structure.
    ///
    /// A [`Cache`] is used to keep track of the insertion order of the messages. The
    /// oldest insertions are at the front of the map, and the newest insertions are at the back of
    /// the map.
    ///
    /// NOTE: For now, this cache is use as "seen cache" to deduplicate messages. We do not store
    /// the message itself.
    cache: Cache<MessageId, ()>,

    /// A table mapping the Topic with the `MessageID` function.
    message_id_fn: HashMap<TopicHash, Rc<MessageIdFn>>,

    /// The service's heartbeat.
    heartbeat: Heartbeat,
}

/// Public API.
impl MessageCacheService {
    /// Creates a new `MessageCache` with the given time-to-live and capacity.
    pub fn new(
        capacity: usize,
        ttl: Duration,
        heartbeat_interval: Duration,
        heartbeat_initial_delay: Duration,
    ) -> Self {
        Self {
            cache: Cache::with_capacity_and_ttl(capacity, ttl),
            message_id_fn: HashMap::new(),
            heartbeat: Heartbeat::new(heartbeat_interval, heartbeat_initial_delay),
        }
    }

    /// Check if the cache contains the given `Message`.
    pub fn contains(&self, message: &Message) -> bool {
        let msg_id = match self.message_id_fn.get(&message.topic()) {
            None => {
                // Unknown topic
                return false;
            }
            Some(id_fn) => id_fn(message),
        };

        self.cache.contains_key(&msg_id)
    }
}

impl Service for MessageCacheService {
    type InEvent = MessageCacheInEvent;
    type OutEvent = ();

    fn on_event(&mut self, ev: Self::InEvent) -> Option<Self::OutEvent> {
        match ev {
            MessageCacheInEvent::Subscribed {
                message_id_fn,
                topic,
            } => {
                // Register the topic's message id function
                let message_id_fn = message_id_fn.unwrap_or(Rc::new(default_message_id_fn));
                self.message_id_fn.insert(topic, message_id_fn);
            }
            MessageCacheInEvent::Unsubscribed(topic) => {
                // Unregister the topic's message id function
                self.message_id_fn.remove(&topic);
            }
            MessageCacheInEvent::MessageReceived(message)
            | MessageCacheInEvent::MessagePublished(message) => {
                let msg_id = match self.message_id_fn.get(&message.topic()) {
                    None => {
                        // Unknown topic
                        return None;
                    }
                    Some(id_fn) => id_fn(&message),
                };

                // Insert message in cache
                self.cache.put(msg_id, ());
            }
        }

        None
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::OutEvent> {
        if self.heartbeat.poll_next_unpin(cx).is_ready() {
            self.cache.clear_expired_entries();
        }

        Poll::Pending
    }
}
