use std::collections::{BTreeSet, HashMap};

use libp2p::identity::PeerId;

use libp2p_pubsub_common::service::{OnEventCtx, Service};
use libp2p_pubsub_core::protocol::{
    ProtocolRouterConnectionEvent, ProtocolRouterInEvent, ProtocolRouterMessageEvent,
    ProtocolRouterOutEvent, ProtocolRouterSubscriptionEvent,
};
use libp2p_pubsub_core::TopicHash;

/// The `Router` struct is the implementation of the [`ProtocolRouter`](
/// libp2p_pubsub_core::protocol::ProtocolRouter) trait for the floodsub protocol.
#[derive(Default)]
pub struct Router {
    /// The topics this router is subscribed to.
    subscriptions: BTreeSet<TopicHash>,

    /// The topics the peers connected to this router are subscribed to and the peers that are
    /// subscribed to them.
    ///
    /// Peers are added to this map when they send the router a message with a topic they are
    /// subscribed to. They are removed on disconnection.
    routing_table: HashMap<TopicHash, BTreeSet<PeerId>>,
}

impl Router {
    /// Track the local node subscription to a topic.
    fn add_subscription(&mut self, topic: TopicHash) -> bool {
        self.subscriptions.insert(topic)
    }

    /// Remove the local node subscription to a topic.
    fn remove_subscription(&mut self, topic: &TopicHash) -> bool {
        self.subscriptions.remove(topic)
    }

    /// Check if the local node is subscribed to a topic.
    fn is_subscribed(&self, topic: &TopicHash) -> bool {
        self.subscriptions.contains(topic)
    }

    /// Add a peer subscription to the routing table.
    ///
    /// The routing table keeps only the peers that are subscribed to a topic that we are
    /// subscribed to, otherwise the peer subscription is ignored.
    ///
    /// Returns `false` if the peer was already subscribed to the topic.
    fn add_peer_subscription(&mut self, peer: PeerId, topic: TopicHash) -> bool {
        self.routing_table.entry(topic).or_default().insert(peer)
    }

    /// Remove a peer subscription from the routing table.
    ///
    /// The routing table keeps only the peers that are subscribed to a topic that we are
    /// subscribed to, otherwise the peer subscription is ignored.
    ///
    /// Returns `true` if the peer was subscribed to the topic.
    fn remove_peer_subscription(&mut self, peer: &PeerId, topic: &TopicHash) -> bool {
        let mut was_subscribed = false;

        if let Some(peers) = self.routing_table.get_mut(topic) {
            was_subscribed = peers.remove(peer);
        }

        if matches!(self.routing_table.get(topic), Some(peers) if peers.is_empty()) {
            self.routing_table.remove(topic);
        }

        was_subscribed
    }

    /// Remove a peer from the routing table.
    ///
    /// When a peer disconnects, we remove it from the routing table, as it is no longer available.
    fn remove_peer(&mut self, peer: &PeerId) {
        for peers in self.routing_table.values_mut() {
            peers.remove(peer);
        }
    }

    /// Get the peers subscribed to a topic.
    ///
    /// Returns a reference to the set of peers subscribed to the topic, if any.
    fn get_peers_subscribed(&self, topic: &TopicHash) -> Option<&BTreeSet<PeerId>> {
        self.routing_table.get(topic)
    }
}

impl Service for Router {
    type InEvent = ProtocolRouterInEvent;
    type OutEvent = ProtocolRouterOutEvent;

    fn on_event<'a>(
        &mut self,
        svc_cx: &mut impl OnEventCtx<'a, Self::OutEvent>,
        ev: Self::InEvent,
    ) {
        match ev {
            ProtocolRouterInEvent::ConnectionEvent(conn_ev) => {
                if let ProtocolRouterConnectionEvent::PeerDisconnected(peer) = conn_ev {
                    self.remove_peer(&peer);
                }
            }
            ProtocolRouterInEvent::SubscriptionEvent(sub_ev) => match sub_ev {
                ProtocolRouterSubscriptionEvent::Subscribed(sub) => {
                    self.add_subscription(sub.topic);
                }
                ProtocolRouterSubscriptionEvent::Unsubscribed(topic) => {
                    self.remove_subscription(&topic);
                }
                ProtocolRouterSubscriptionEvent::PeerSubscribed { peer, topic } => {
                    self.add_peer_subscription(peer, topic);
                }
                ProtocolRouterSubscriptionEvent::PeerUnsubscribed { peer, topic } => {
                    self.remove_peer_subscription(&peer, &topic);
                }
            },
            ProtocolRouterInEvent::MessageEvent(ProtocolRouterMessageEvent::MessageReceived {
                src,
                message,
                ..
            }) => {
                let topic = message.topic();
                if !self.is_subscribed(&topic) {
                    return;
                }

                if let Some(peers) = self.get_peers_subscribed(&topic) {
                    let peers = peers
                        .iter()
                        .filter(|p| **p != src)
                        .cloned()
                        .collect::<Vec<_>>();
                    if peers.is_empty() {
                        return;
                    }

                    svc_cx.emit(ProtocolRouterOutEvent::ForwardMessage {
                        dest: peers,
                        message,
                    });
                }
            }
            ProtocolRouterInEvent::MessageEvent(ProtocolRouterMessageEvent::MessagePublished {
                message,
                ..
            }) => {
                let topic = message.topic();
                if !self.is_subscribed(&topic) {
                    return;
                }

                if let Some(peers) = self.get_peers_subscribed(&topic) {
                    let peers = peers.iter().cloned().collect::<Vec<_>>();
                    svc_cx.emit(ProtocolRouterOutEvent::ForwardMessage {
                        dest: peers,
                        message,
                    });
                } else {
                    tracing::debug!("No peers subscribed to topic: {:?}", topic);
                }
            }
            ProtocolRouterInEvent::ControlEvent(_) => {
                // No-op
            }
        }
    }
}
