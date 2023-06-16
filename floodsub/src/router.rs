use std::collections::{BTreeSet, HashMap};

use libp2p::identity::PeerId;

use crate::topic::TopicHash;

#[derive(Default)]
pub struct Router {
    /// The topics this router is subscribed to.
    subscriptions: BTreeSet<TopicHash>,

    /// The peers this router is connected to and the topics they are subscribed to.
    ///
    /// Peers are added to this map when they send the router a message with a topic they are
    /// subscribed to. They are removed on disconnection.
    ///
    /// This is the reverse of `topics_to_peers`.
    peers_to_topics: HashMap<PeerId, BTreeSet<TopicHash>>,

    /// The topics the peers connected to this router are subscribed to and the peers that are
    /// subscribed to them.
    ///
    /// Peers are added to this map when they send the router a message with a topic they are
    /// subscribed to. They are removed on disconnection.
    ///
    /// This is the reverse of `peers_to_topics`.
    topics_to_peers: HashMap<TopicHash, BTreeSet<PeerId>>,
}

/// Subscription management.
impl Router {
    /// Whether the router is subscribed to the given topic or not.
    pub fn is_subscribed(&self, topic: &TopicHash) -> bool {
        self.subscriptions.contains(topic)
    }

    /// Subscribes the router to the given topic.
    ///
    /// Returns `false` if the router was already subscribed to the topic.
    pub fn subscribe(&mut self, topic: TopicHash) -> bool {
        self.subscriptions.insert(topic)
    }

    /// Unsubscribes the router from the given topic.
    ///
    /// Returns `false` if the router was not subscribed to the topic.
    pub fn unsubscribe(&mut self, topic: &TopicHash) -> bool {
        self.subscriptions.remove(topic)
    }

    /// Returns the topics the router is subscribed to.
    pub fn subscriptions(&self) -> impl Iterator<Item = &TopicHash> {
        self.subscriptions.iter()
    }
}

/// PeerId to Subscription tracking.s
impl Router {
    /// Adds a peer to the router and tracks it subscription to the given topic.
    pub fn add_peer_subscription(&mut self, peer: PeerId, topic: TopicHash) {
        self.peers_to_topics
            .entry(peer)
            .or_default()
            .insert(topic.clone());

        self.topics_to_peers.entry(topic).or_default().insert(peer);
    }

    /// Adds a peer to the router and subscribes it to the given topics.
    pub fn add_peer_subscriptions(
        &mut self,
        peer: PeerId,
        topics: impl IntoIterator<Item = TopicHash>,
    ) {
        for topic in topics {
            self.add_peer_subscription(peer, topic);
        }
    }

    /// Removes a peer from the router and unsubscribes it from the given topic.
    ///
    /// If the peer is no longer subscribed to any topic, it is removed from the router. Similarly,
    /// if the topic is no longer subscribed to by any peer, it is removed from the router.
    pub fn remove_peer_subscription(&mut self, peer: &PeerId, topic: &TopicHash) {
        if let Some(topics) = self.peers_to_topics.get_mut(peer) {
            topics.remove(topic);
        }

        if let Some(peers) = self.topics_to_peers.get_mut(topic) {
            peers.remove(peer);
        }

        // If the peer is no longer subscribed to any topic, remove it from the router.
        if matches!(self.peers_to_topics.get(peer), Some(topics) if topics.is_empty()) {
            self.peers_to_topics.remove(peer);
        }

        // If the topic is no longer subscribed to by any peer, remove it from the router.
        if matches!(self.topics_to_peers.get(topic), Some(peers) if peers.is_empty()) {
            self.topics_to_peers.remove(topic);
        }
    }

    /// Removes a peer from the router and unsubscribes it from the given topics.
    pub fn remove_peer(&mut self, peer: &PeerId) {
        if let Some(topics) = self.peers_to_topics.remove(peer) {
            for topic in topics {
                self.remove_peer_subscription(peer, &topic);
            }
        }
    }

    /// Get the topics a peer is subscribed to.
    pub fn get_peer_subscriptions(&self, peer: &PeerId) -> Option<&BTreeSet<TopicHash>> {
        self.peers_to_topics.get(peer)
    }

    /// Get the peers subscribed to a topic.
    pub fn get_subscription_peers(&self, topic: &TopicHash) -> Option<&BTreeSet<PeerId>> {
        self.topics_to_peers.get(topic)
    }
}

/// Routing and propagation.
impl Router {
    /// Get the peers to propagate a message to for a given topic. In floodsub messages should
    /// be forwarded to all known peers subscribed to the topic.
    ///
    /// **Note:** The returned set of peers may include the peer that propagated the message.
    pub fn propagation_routes(&self, topic: &TopicHash) -> impl IntoIterator<Item = PeerId> {
        debug_assert!(self.is_subscribed(topic), "Not subscribed to topic");

        if let Some(peers) = self.get_subscription_peers(topic) {
            return peers.clone().into_iter();
        }

        BTreeSet::<PeerId>::new().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_peer() -> PeerId {
        PeerId::random()
    }

    fn test_topic() -> TopicHash {
        TopicHash::from_raw(format!("/test/topic-{}", rand::random::<u64>()))
    }

    fn test_router() -> Router {
        Default::default()
    }

    mod subscriptions {
        use super::*;

        #[test]
        fn subscribe_to_topic() {
            //// Given
            let topic = test_topic();

            let mut router = test_router();

            //// When
            let subscribed = router.subscribe(topic.clone());

            //// Then
            assert!(subscribed);
            assert!(router.is_subscribed(&topic));
        }

        #[test]
        fn unsubscribe_from_topic() {
            //// Given
            let topic = test_topic();

            let mut router = test_router();
            router.subscribe(topic.clone());

            //// When
            let unsubscribed = router.unsubscribe(&topic);

            //// Then
            assert!(unsubscribed);
            assert!(!router.is_subscribed(&topic));
        }

        #[test]
        fn subscribe_when_already_subscribed() {
            //// Given
            let topic = test_topic();

            let mut router = test_router();
            router.subscribe(topic.clone());

            //// When
            let subscribed = router.subscribe(topic.clone());

            //// Then
            assert!(!subscribed);
            assert!(router.is_subscribed(&topic));
        }

        #[test]
        fn unsubscribe_when_not_subscribed() {
            //// Given
            let topic = test_topic();

            let mut router = test_router();

            //// When
            let unsubscribed = router.unsubscribe(&topic);

            //// Then
            assert!(!unsubscribed);
            assert!(!router.is_subscribed(&topic));
        }
    }

    mod peer_subscriptions {
        use assert_matches::assert_matches;

        use super::*;

        #[test]
        fn add_peer_subscription() {
            //// Given
            let peer = test_peer();
            let topic = test_topic();

            let mut router = test_router();

            //// When
            router.add_peer_subscription(peer, topic.clone());

            //// Then
            assert_matches!(router.get_peer_subscriptions(&peer), Some(subscriptions) => {
                assert!(subscriptions.contains(&topic));
            });
            assert_matches!(router.get_subscription_peers(&topic), Some(peers) => {
                assert!(peers.contains(&peer));
            });
        }

        #[test]
        fn add_peer_multiple_subscriptions() {
            //// Given
            let peer = test_peer();
            let topics = vec![test_topic(), test_topic()];

            let mut router = test_router();

            //// When
            router.add_peer_subscriptions(peer, topics.clone());

            //// Then
            // It should contain topics[0] topic.
            assert_matches!(router.get_peer_subscriptions(&peer), Some(subscriptions) => {
                assert!(subscriptions.contains(&topics[0]));
            });
            assert_matches!(router.get_subscription_peers(&topics[0]), Some(peers) => {
                assert!(peers.contains(&peer));
            });

            // It should contain topics[1] topic.
            assert_matches!(router.get_peer_subscriptions(&peer), Some(subscriptions) => {
                assert!(subscriptions.contains(&topics[1]));
            });
            assert_matches!(router.get_subscription_peers(&topics[1]), Some(peers) => {
                assert!(peers.contains(&peer));
            });
        }

        #[test]
        fn remove_single_peer_subscription() {
            //// Given
            let peer = test_peer();
            let topics = vec![test_topic(), test_topic()];

            let mut router = test_router();
            router.add_peer_subscriptions(peer, topics.clone());

            //// When
            router.remove_peer_subscription(&peer, &topics[0]);

            //// Then
            // It should NOT contain topics[0] topic.
            // It should contain topics[1] topic.
            assert_matches!(router.get_peer_subscriptions(&peer), Some(subscriptions) => {
                assert!(!subscriptions.contains(&topics[0]));
                assert!(subscriptions.contains(&topics[1]));
            });

            assert!(router.get_subscription_peers(&topics[0]).is_none());
            assert_matches!(router.get_subscription_peers(&topics[1]), Some(peers) => {
                assert!(peers.contains(&peer));
            });
        }

        #[test]
        fn remove_all_peer_subscriptions() {
            //// Given
            let peer = test_peer();
            let topic = test_topic();

            let mut router = test_router();
            router.add_peer_subscription(peer, topic.clone());

            //// When
            router.remove_peer_subscription(&peer, &topic);

            //// Then
            assert!(router.get_peer_subscriptions(&peer).is_none());
            assert!(router.get_subscription_peers(&topic).is_none());
        }

        #[test]
        fn remove_unknown_topic_subscription() {
            //// Given
            let peer = test_peer();
            let topic = test_topic();

            let mut router = test_router();
            router.add_peer_subscription(peer, topic.clone());

            //// When
            let unknown_topic = test_topic();
            router.remove_peer_subscription(&peer, &unknown_topic);

            //// Then
            assert_matches!(router.get_peer_subscriptions(&peer), Some(subscriptions) => {
                assert!(subscriptions.contains(&topic));
            });
        }

        #[test]
        fn remove_unknown_peer_subscription() {
            //// Given
            let peer = test_peer();
            let topic = test_topic();

            let mut router = test_router();
            router.add_peer_subscription(peer, topic.clone());

            //// When
            let unknown_peer = test_peer();
            router.remove_peer_subscription(&unknown_peer, &topic);

            //// Then
            assert_matches!(router.get_peer_subscriptions(&peer), Some(subscriptions) => {
                assert!(subscriptions.contains(&topic));
            });
        }

        #[test]
        fn remove_peer() {
            //// Given
            let peer_a = test_peer();
            let peer_b = test_peer();
            let topic = test_topic();

            let mut router = test_router();
            router.add_peer_subscription(peer_a, topic.clone());
            router.add_peer_subscription(peer_b, topic.clone());

            //// When
            router.remove_peer(&peer_a);

            //// Then
            assert!(router.get_peer_subscriptions(&peer_a).is_none());
            assert!(router.get_peer_subscriptions(&peer_b).is_some());

            assert_matches!(router.get_subscription_peers(&topic), Some(peers) => {
                assert!(!peers.contains(&peer_a));
                assert!(peers.contains(&peer_b));
            });
        }
    }

    mod routing {
        use super::*;

        #[test]
        fn floodsub_propagation_routes() {
            //// Given
            let peer_a = test_peer();
            let peer_b = test_peer();
            let peer_c = test_peer();
            let topic_a = test_topic();
            let topic_b = test_topic();
            let topic_c = test_topic();

            let topic = test_topic();

            let mut router = test_router();
            router.subscribe(topic.clone());

            router.add_peer_subscriptions(peer_a, vec![topic_a, topic.clone()]);
            router.add_peer_subscriptions(peer_b, vec![topic_b, topic.clone()]);
            router.add_peer_subscriptions(peer_c, vec![topic_c]);

            //// When
            let routes = router.propagation_routes(&topic);

            //// Then
            let routes = routes.into_iter().collect::<Vec<_>>();
            assert_eq!(routes.len(), 2);
            assert!(routes.contains(&peer_a));
            assert!(routes.contains(&peer_b));
            assert!(!routes.contains(&peer_c));
        }

        #[test]
        fn floodsub_propagation_routes_no_peers() {
            //// Given
            let peer_a = test_peer();
            let peer_b = test_peer();
            let topic_a = test_topic();
            let topic_b = test_topic();

            let topic = test_topic();

            let mut router = test_router();
            router.subscribe(topic.clone());

            router.add_peer_subscription(peer_a, topic_b);
            router.add_peer_subscription(peer_b, topic_a);

            //// When
            let routes = router.propagation_routes(&topic);

            //// Then
            let routes = routes.into_iter().collect::<Vec<_>>();
            assert!(routes.is_empty());
        }
    }
}
