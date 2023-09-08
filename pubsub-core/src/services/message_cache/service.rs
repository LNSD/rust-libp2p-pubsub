use std::collections::HashMap;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::StreamExt;

use libp2p_pubsub_common::heartbeat::Heartbeat;
use libp2p_pubsub_common::service::{PollCtx, Service};
use libp2p_pubsub_common::ttl_cache::Cache;

use crate::framing::Message;
use crate::message_id::{default_message_id_fn, MessageId, MessageIdFn};
use crate::topic::TopicHash;

use super::events::{ServiceIn, SubscriptionEvent};

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
    type InEvent = ServiceIn;
    type OutEvent = ();

    fn poll(
        &mut self,
        svc_cx: PollCtx<'_, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        let (mut in_cx, _out_cx) = svc_cx.split();

        // Poll the heartbeat stream.
        if self.heartbeat.poll_next_unpin(cx).is_ready() {
            self.cache.clear_expired_entries();
        }

        // Process all the service input mailbox events.
        while let Some(ev) = in_cx.pop_next() {
            match ev {
                ServiceIn::SubscriptionEvent(sub_ev) => match sub_ev {
                    SubscriptionEvent::Subscribed {
                        message_id_fn,
                        topic,
                    } => {
                        // Register the topic's message id function
                        let message_id_fn = message_id_fn.unwrap_or(Rc::new(default_message_id_fn));
                        self.message_id_fn.insert(topic, message_id_fn);
                    }
                    SubscriptionEvent::Unsubscribed(topic) => {
                        // Unregister the topic's message id function
                        self.message_id_fn.remove(&topic);
                    }
                },
                ServiceIn::MessageReceived(message) | ServiceIn::MessagePublished(message) => {
                    let msg_id = match self.message_id_fn.get(&message.topic()) {
                        None => {
                            // Unknown topic
                            continue;
                        }
                        Some(id_fn) => id_fn(&message),
                    };

                    // Insert message in cache
                    self.cache.put(msg_id, ());
                }
            }
        }

        Poll::Pending
    }
}
