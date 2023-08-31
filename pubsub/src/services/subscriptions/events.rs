use libp2p::identity::PeerId;

use crate::framing::SubscriptionAction;
use crate::subscription::Subscription;
use crate::topic::TopicHash;

/// Events consumed by the [`SubscriptionsService`].
#[derive(Debug, Clone)]
pub enum ServiceIn {
    /// A local subscription request.
    ///
    /// This event is emitted when the pub-sub network behaviour [`subscribe`] method is called.
    LocalSubscriptionRequest(Subscription),
    /// A local unsubscription request.
    ///
    /// This event is emitted when the pub-sub network behaviour [`unsubscribe`] method is called.
    LocalUnsubscriptionRequest(TopicHash),
    /// A peer subscription request received.
    PeerSubscriptionRequest {
        /// Peer that sent the subscription request.
        src: PeerId,
        /// Subscription action.
        action: SubscriptionAction,
    },
    /// A peer connection event.
    PeerConnectionEvent(PeerConnectionEvent),
}

impl ServiceIn {
    /// Builds a `ServiceIn::PeerConnectionEvent` event.
    pub fn from_peer_connection_event(ev: impl Into<PeerConnectionEvent>) -> Self {
        ServiceIn::PeerConnectionEvent(ev.into())
    }
}

/// Peer connection events consumed by the [`SubscriptionsService`].
#[derive(Debug, Clone)]
pub enum PeerConnectionEvent {
    /// New peer connected.
    NewPeerConnected(PeerId),
    /// Peer disconnected.
    PeerDisconnected(PeerId),
}

/// Events emitted by the [`SubscriptionsService`].
#[derive(Debug, Clone)]
pub enum ServiceOut {
    /// New local subscription.
    ///
    /// This event is emitted when the node subscribes to a topic. This will emit one subscription
    /// request to each active peer.
    Subscribed(Subscription),
    /// Local unsubscription.
    ///
    /// This event is emitted when the node unsubscribes from a topic. This will emit one
    /// unsubscription request to each active peer.
    Unsubscribed(TopicHash),
    /// A peer registered a new subscription.
    ///
    /// This peer is now subscribed to the `topic`.
    PeerSubscribed {
        /// Peer that subscribed.
        peer: PeerId,

        /// Topic that the peer subscribed to.
        topic: TopicHash,
    },
    /// A peer unregistered a subscription.
    ///
    /// This peer is no longer subscribed to the `topic`.
    PeerUnsubscribed {
        /// Peer that unsubscribed.
        peer: PeerId,

        /// Topic that the peer unsubscribed from.
        topic: TopicHash,
    },
    /// Send all the local node subscriptions to a peer.
    ///
    /// This event is emitted when a new peer connects to the node. This will send one
    /// [`SubscriptionAction::Subscribe`] action per topic that the local node is subscribed to.
    SendSubscriptions {
        /// Peer to send the subscriptions to.
        peer: PeerId,

        /// Topics list to send.
        topics: Vec<TopicHash>,
    },
}
