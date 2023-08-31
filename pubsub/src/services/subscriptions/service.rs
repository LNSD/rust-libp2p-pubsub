use std::collections::{BTreeSet, HashMap};

use libp2p::PeerId;

use common::service::Service;

use crate::framing::SubscriptionAction;
use crate::services::subscriptions::PeerConnectionEvent;
use crate::topic::TopicHash;

use super::events::{ServiceIn, ServiceOut};

#[derive(Debug, Default)]
pub struct SubscriptionsService {
    /// The topics this node is subscribed to.
    local_subscriptions: BTreeSet<TopicHash>,

    /// The peers this router is connected to and the topics they are subscribed to.
    ///
    /// Peers are added to this map when they send the router a message with a topic they are
    /// subscribed to. They are removed on disconnection.
    peers_subscriptions: HashMap<PeerId, BTreeSet<TopicHash>>,
}

/// Public API.
impl SubscriptionsService {
    /// Whether the router is subscribed to the given topic or not.
    pub fn is_subscribed(&self, topic: &TopicHash) -> bool {
        self.local_subscriptions.contains(topic)
    }

    /// Returns the topics this node is subscribed to.
    pub fn subscriptions(&self) -> &BTreeSet<TopicHash> {
        &self.local_subscriptions
    }

    /// Returns whether the given peer is subscribed to the given topic or not.
    ///
    /// If the peer is not connected, this returns `None`.
    #[cfg(test)]
    pub fn is_peer_subscribed(&self, peer: &PeerId, topic: &TopicHash) -> Option<bool> {
        self.peers_subscriptions
            .get(peer)
            .map(|topics| topics.contains(topic))
    }

    /// Returns the topics the given peer is subscribed to.
    ///
    /// If the peer is not connected, this returns `None`.
    pub fn peer_subscriptions(&self, peer: &PeerId) -> Option<&BTreeSet<TopicHash>> {
        self.peers_subscriptions.get(peer)
    }
}

// Internal API.
impl SubscriptionsService {
    /// Adds a new local subscription.
    ///
    /// If the node was not already subscribed to the topic, this returns `true`. Otherwise, it
    /// returns `false`.
    fn add_local_subscription(&mut self, topic: TopicHash) -> bool {
        self.local_subscriptions.insert(topic)
    }

    /// Removes a local subscription.
    ///
    /// If the node was subscribed to the topic, this returns `true`. Otherwise, it returns
    /// `false`.
    fn remove_local_subscription(&mut self, topic: TopicHash) -> bool {
        self.local_subscriptions.remove(&topic)
    }

    /// Adds a new peer subscription.
    ///
    /// If the peer was not already subscribed to the topic, this returns `true`. Otherwise, it
    /// returns `false`.
    fn add_peer_subscription(&mut self, peer: PeerId, topic: TopicHash) -> bool {
        let peer_subscriptions = self.peers_subscriptions.entry(peer).or_default();
        peer_subscriptions.insert(topic.clone())
    }

    /// Removes a peer subscription.
    ///
    /// If the peer was subscribed to the topic, this returns `true`. Otherwise, it returns `false`.
    fn remove_peer_subscription(&mut self, peer: &PeerId, topic: &TopicHash) -> bool {
        if let Some(peer_subscriptions) = self.peers_subscriptions.get_mut(peer) {
            return peer_subscriptions.remove(topic);
        }

        false
    }

    /// Removes a peer from the peer subscriptions tracker.
    fn remove_peer(&mut self, peer: &PeerId) {
        self.peers_subscriptions.remove(peer);
    }
}

impl Service for SubscriptionsService {
    type InEvent = ServiceIn;
    type OutEvent = ServiceOut;

    fn on_event(&mut self, ev: Self::InEvent) -> Option<Self::OutEvent> {
        match ev {
            ServiceIn::LocalSubscriptionRequest(sub) => {
                // Emit a [`SubscriptionsOutEvent::Subscribed`] event if the node was not already
                // subscribed to the topic.
                if self.add_local_subscription(sub.topic.clone()) {
                    return Some(ServiceOut::Subscribed(sub));
                }
            }
            ServiceIn::LocalUnsubscriptionRequest(topic) => {
                // Emit a [`SubscriptionsOutEvent::Unsubscribed`] event if the node was subscribed to the
                // topic.
                if self.remove_local_subscription(topic.clone()) {
                    return Some(ServiceOut::Unsubscribed(topic));
                }
            }
            ServiceIn::PeerSubscriptionRequest { src: peer, action } => match action {
                SubscriptionAction::Subscribe(topic) => {
                    // Emit a [`SubscriptionsOutEvent::PeerSubscribed`] event if the peer was not already
                    // subscribed to the topic.
                    if self.add_peer_subscription(peer, topic.clone()) {
                        return Some(ServiceOut::PeerSubscribed { peer, topic });
                    }
                }
                SubscriptionAction::Unsubscribe(topic) => {
                    // Emit a [`SubscriptionsOutEvent::PeerUnsubscribed`] event if the peer was subscribed to
                    // the topic.
                    if self.remove_peer_subscription(&peer, &topic) {
                        return Some(ServiceOut::PeerUnsubscribed { peer, topic });
                    }
                }
            },
            ServiceIn::PeerConnectionEvent(conn_ev) => match conn_ev {
                PeerConnectionEvent::NewPeerConnected(peer) => {
                    // Send all the local node subscriptions to a peer when it connects for the first
                    // time (only if the node is subscribed to at least one topic).
                    if self.local_subscriptions.is_empty() {
                        return None;
                    }

                    let topics = self.local_subscriptions.iter().cloned().collect::<Vec<_>>();
                    return Some(ServiceOut::SendSubscriptions { peer, topics });
                }
                PeerConnectionEvent::PeerDisconnected(peer) => {
                    // Remove the peer from the peer subscriptions tracker when it disconnects.
                    self.remove_peer(&peer);
                }
            },
        }

        None
    }
}
