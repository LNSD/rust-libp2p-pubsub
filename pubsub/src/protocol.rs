use std::rc::Rc;

use libp2p::PeerId;

use common::service::Service;

use crate::framing::Message;
use crate::subscription::Subscription;
use crate::topic::TopicHash;

/// The pubsub protocol trait.
///
/// This trait is used by the [`Behaviour`] to identify the pubsub protocol and create a [`Handler`]
/// instance for the protocol as well as the [`ProtocolRouter`] instance responsible for routing
/// messages to the appropriate peers.
pub trait Protocol: 'static {
    type ProtocolId: ProtocolId;
    type RouterService: ProtocolRouter;

    /// Returns the protocol's ID string.
    ///
    /// See [`ProtocolId`] for more information.
    // TODO: Revisit this after refactoring the Pubsub connection handler.
    fn protocol_id() -> &'static str {
        Self::ProtocolId::PROTOCOL_ID
    }

    /// Returns the protocol's router service.
    ///
    /// See [`ProtocolRouter`] for more information.
    fn router(&self) -> Self::RouterService;
}

/// The protocol id trait.
///
/// This trait is used by the [`Behaviour`] to identify the pubsub protocol and create a [`Handler`]
/// instance for the protocol
pub trait ProtocolId {
    // TODO: Revisit this after refactoring the Pubsub connection handler.
    const PROTOCOL_ID: &'static str;
}

/// A pubsub protocol router input event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterInEvent {
    /// A connection event.
    ConnectionEvent(ProtocolRouterConnectionEvent),
    /// A subscription event.
    SubscriptionEvent(ProtocolRouterSubscriptionEvent),
    /// A was message received from a peer.
    MessageReceived {
        /// The message propagator.
        src: PeerId,
        /// The message.
        message: Rc<Message>,
    },
    /// A message ready to publish.
    ///
    /// This event is generated by the `publish` method of the pubsub behaviour.
    MessagePublished(Rc<Message>),
}

/// A pubsub protocol router connection event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterConnectionEvent {
    /// A new peer connected.
    PeerConnected(PeerId),
    /// A peer disconnected.
    PeerDisconnected(PeerId),
}

/// A pubsub protocol router topic subscription event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterSubscriptionEvent {
    /// A subscription event.
    Subscribed(Subscription),
    /// An unsubscription event.
    Unsubscribed(TopicHash),
    /// A peer subscribed to a topic.
    PeerSubscribed { peer: PeerId, topic: TopicHash },
    /// A peer unsubscribed from a topic.
    PeerUnsubscribed { peer: PeerId, topic: TopicHash },
}

/// A pubsub protocol router output event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterOutEvent {
    /// Forward the message to the given peers.
    ForwardMessage {
        /// The destination peers.
        dest: Vec<PeerId>,
        /// The message.
        message: Rc<Message>,
    },
}

// NOTE: Use `trait_set` crate as `trait_alias` is not yet stable.
//       https://github.com/rust-lang/rust/issues/41517
trait_set::trait_set! {
    /// The protocol message router service trait.
    ///
    /// This trait is used by the [`Behaviour`] to route received (and published) messages
    /// to the appropriate peers.
    ///
    /// It handles the [`ProtocolRouterInEvent`] and generates [`ProtocolRouterOutEvent`] events.
    pub trait ProtocolRouter = Service<InEvent = ProtocolRouterInEvent, OutEvent = ProtocolRouterOutEvent>;
}