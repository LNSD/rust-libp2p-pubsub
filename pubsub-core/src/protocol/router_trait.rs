use std::rc::Rc;

use libp2p::PeerId;

use libp2p_pubsub_common::service::Service;

use crate::framing::{ControlMessage, Message as FrameMessage};
use crate::message_id::MessageId;
use crate::subscription::Subscription;
use crate::topic::TopicHash;

/// A pubsub protocol router input event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterInEvent {
    /// A connection event.
    ConnectionEvent(ProtocolRouterConnectionEvent),
    /// A subscription event.
    SubscriptionEvent(ProtocolRouterSubscriptionEvent),
    /// A message event.
    MessageEvent(ProtocolRouterMessageEvent),
    /// A pubsub control message event.
    ControlEvent(ProtocolRouterControlEvent),
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

/// A pubsub protocol router message event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterMessageEvent {
    /// A message was received from a peer.
    MessageReceived {
        /// The message propagator.
        src: PeerId,
        /// The message.
        message: Rc<FrameMessage>,
        /// The message id.
        message_id: MessageId,
    },
    /// A message ready to publish.
    ///
    /// This event is generated by the `publish` method of the pubsub behaviour.
    MessagePublished {
        /// The message.
        message: Rc<FrameMessage>,
        /// The message id.
        message_id: MessageId,
    },
}

/// A pubsub protocol control message event.
#[derive(Debug, Clone)]
pub struct ProtocolRouterControlEvent {
    /// The message source.
    pub(crate) src: PeerId,
    /// The message.
    pub(crate) message: ControlMessage,
}

/// A pubsub protocol router output event.
#[derive(Debug, Clone)]
pub enum ProtocolRouterOutEvent {
    /// Forward the message to the given peers.
    ForwardMessageASSS {
        /// The destination peers.
        dest: Vec<PeerId>,
        /// The message.
        message: Rc<FrameMessage>,
    },
    /// Send control message to the given peer.
    SendControlMessage {
        /// The destination peer.
        dest: PeerId,
        // The control message.
        message: ControlMessage,
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
