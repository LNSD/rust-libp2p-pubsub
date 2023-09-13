use std::task::{Context, Poll};
use std::time::Duration;

use futures::StreamExt;

use libp2p_pubsub_common::heartbeat::Heartbeat;
use libp2p_pubsub_common::service::{InCtx, PollCtx, Service};
use libp2p_pubsub_common::ttl_cache::Cache;

use crate::message_id::MessageId;
use crate::services::message_cache::events::MessageEvent;

use super::events::ServiceIn;

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
            heartbeat: Heartbeat::new(heartbeat_interval, heartbeat_initial_delay),
        }
    }

    /// Check if the cache contains the given `Message`.
    pub fn contains(&self, message_id: &MessageId) -> bool {
        self.cache.contains_key(message_id)
    }

    /// Get the cache usage.
    ///
    /// This is the number of messages currently in the cache.
    #[cfg(test)]
    pub fn usage(&self) -> usize {
        self.cache.len()
    }
}

impl Service for MessageCacheService {
    type InEvent = ServiceIn;
    type OutEvent = ();

    fn poll<'a>(
        &mut self,
        svc_cx: impl PollCtx<'a, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        let (mut in_cx, _out_cx) = svc_cx.split();

        // Poll the heartbeat stream.
        if self.heartbeat.poll_next_unpin(cx).is_ready() {
            self.cache.clear_expired_entries();
        }

        // Process the incoming events.
        while let Some(ev) = in_cx.pop_next() {
            match ev {
                ServiceIn::MessageEvent(MessageEvent::MessageReceived { message_id, .. }) => {
                    // Insert message into the cache
                    self.cache.put(message_id, ());
                }
                ServiceIn::MessageEvent(MessageEvent::MessagePublished { message_id, .. }) => {
                    // Insert message into the cache
                    self.cache.put(message_id, ());
                }
            }
        }

        Poll::Pending
    }
}
