#![allow(dead_code)] // TODO: Remove after the service integration

use std::rc::Rc;

use libp2p::identity::PeerId;

use crate::framing::{FrameProto, Message as FrameMessage, SubscriptionAction};

/// The input event for the framing service.
#[derive(Debug, Clone)]
pub enum ServiceIn {
    /// An event originated from the connection handler to the network behaviour.
    Upstream(UpstreamInEvent),
    /// An event originated inthe network behaviour to the connection handler.
    Downstream(DownstreamInEvent),
}

/// The output event for the framing service.
#[derive(Debug, Clone)]
pub enum ServiceOut {
    /// An event originated from the connection handler to the network behaviour.
    Upstream(UpstreamOutEvent),
    /// An event originated from the network behaviour to the connection handler.
    Downstream(DownstreamOutEvent),
}

#[derive(Debug, Clone)]
pub enum UpstreamInEvent {
    /// A raw frame received from the `src` peer.
    ///
    /// This event is emitted by the connection handler when a new raw frame is received.
    RawFrameReceived {
        /// The peer that propagated the frame.
        src: PeerId,
        /// The raw frame.
        frame: FrameProto, //TODO: Replace with bytes after the conn_handler refactoring
    },
}

#[derive(Debug, Clone)]
pub enum UpstreamOutEvent {
    /// A message forwarded by the `src` peer.
    MessageReceived {
        /// The peer that propagated the message.
        src: PeerId,
        /// The frame message.
        ///
        /// This message is the result of validating and decoding the raw frame.
        message: Rc<FrameMessage>,
    },
    /// A subscription action request received by the `src` peer.
    SubscriptionRequestReceived {
        /// The peer that propagated the message.
        src: PeerId,
        /// A peer's subscription action request.
        action: SubscriptionAction,
    },
}

#[derive(Debug, Clone)]
pub enum DownstreamInEvent {
    /// A message to be forwarded to the `dest` peer.
    ForwardMessage {
        /// THe destination peer.
        dest: PeerId,
        /// The message to propagate.
        message: Rc<FrameMessage>,
    },
    /// A subscription action to be sent to the `dest` peer.
    SendSubscriptionRequest {
        /// The destination peer.
        dest: PeerId,
        /// The subscription actions to send.
        action: Vec<SubscriptionAction>,
    },
}

#[derive(Debug, Clone)]
pub enum DownstreamOutEvent {
    /// A raw frame to be sent to the `dest` peer.
    SendFrame {
        /// The destination peer.
        dest: PeerId,
        /// The raw frame to propagate.
        frame: FrameProto, // TODO: Replace with bytes after the conn_handler refactoring
    },
}
