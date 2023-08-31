use std::hash::Hash;
use std::time::Duration;

use hashlink::linked_hash_map::{LinkedHashMap, RawEntryMut};
use instant::Instant;

struct CacheEntry<M> {
    /// The timestamp at which the message was received.
    pub timestamp: Instant,
    /// The message.
    pub message: M,
}

/// Cache of messages that we have already seen.
///
/// This is used to avoid sending the same message multiple times.
pub struct Cache<K, V> {
    /// Maximum number of messages in the cache.
    capacity: usize,

    /// Time-to-live of messages in the cache.
    ttl: Duration,

    /// The internal cache data structure.
    ///
    /// A `LinkedHashMap` is used to keep track of the insertion order of the messages. The
    /// oldest insertions are at the front of the map, and the newest insertions are at the back of
    /// the map.
    cache: LinkedHashMap<K, CacheEntry<V>>,
}

impl<K, V> Default for Cache<K, V> {
    /// Creates a new empty cache.
    ///
    /// Capacity defaults to 1024 messages and a time-to-live to 5 seconds.
    fn default() -> Self {
        Self::with_capacity_and_ttl(1024, Duration::from_secs(5))
    }
}

impl<K, V> Cache<K, V> {
    /// Creates a new empty cache with the given capacity and time-to-live.
    #[must_use]
    pub fn with_capacity_and_ttl(capacity: usize, ttl: Duration) -> Self {
        Self {
            capacity,
            ttl,
            cache: LinkedHashMap::with_capacity(capacity),
        }
    }
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Inserts a message in the cache.
    ///
    /// Returns `true` if the message was not already in the cache. Returns `false` if the message
    /// was already in the cache.
    ///
    /// If the source is `None`, then the message is assumed to have been sent by us.
    pub fn put(&mut self, id: K, message: V) -> bool {
        let result = match self.cache.raw_entry_mut().from_key(&id) {
            RawEntryMut::Occupied(mut entry) => {
                // If the entry has expired but it is still present, update the timestamp
                // and pretend that the entry was not already in the cache.
                let was_expired = entry.get().timestamp.elapsed() > self.ttl;

                // Update the insertion time of the entry and push it to the back of the map.
                entry.get_mut().timestamp = Instant::now();
                entry.to_back();

                // If entry was expired but it has been refreshed, return `true`.
                was_expired
            }
            RawEntryMut::Vacant(entry) => {
                let timestamp = Instant::now();
                entry.insert(id, CacheEntry { timestamp, message });

                true
            }
        };

        // If the cache is full, remove the oldest message.
        if self.cache.len() > self.capacity {
            self.cache.pop_front();
        }

        result
    }

    /// Returns an iterator over all the entries of the cache (expired and not-expired).
    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.cache.iter().map(|(id, entry)| (id, &entry.message))
    }

    /// Returns the number of non-expired messages in the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache
            .iter()
            .skip_while(|(_, entry)| entry.timestamp.elapsed() > self.ttl)
            .count()
    }

    /// Returns `true` if the cache contains a non-expired message with the given ID.
    #[must_use]
    pub fn contains_key(&self, id: &K) -> bool {
        self.cache
            .get(id)
            .is_some_and(|entry| entry.timestamp.elapsed() <= self.ttl)
    }

    /// Returns a reference to the message with the given ID, if it exists in the cache and has not
    /// expired.
    #[must_use]
    pub fn get(&self, id: &K) -> Option<&V> {
        self.cache
            .get(id)
            .filter(|entry| entry.timestamp.elapsed() <= self.ttl)
            .map(|entry| &entry.message)
    }

    /// Removes the message with the given ID from the cache.
    ///
    /// Returns the removed cache entry, if it existed in the cache and had not expired.
    pub fn remove(&mut self, id: &K) -> Option<V> {
        self.cache
            .remove(id)
            .filter(|entry| entry.timestamp.elapsed() <= self.ttl)
            .map(|entry| entry.message)
    }

    /// Remove all expired messages from the cache.
    ///
    /// An entry is considered expired if the elapsed time since the insertion of the entry is
    /// greater than the time-to-live of the cache, then the entry is considered expired.
    pub fn clear_expired_entries(&mut self) {
        let mut to_remove = Vec::new();

        for (id, entry) in self.cache.iter() {
            if entry.timestamp.elapsed() <= self.ttl {
                break;
            }

            to_remove.push(id.clone());
        }

        for id in to_remove {
            self.cache.remove(&id);
        }
    }
}
