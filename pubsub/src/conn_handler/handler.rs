use std::collections::VecDeque;
use std::convert::Infallible;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use asynchronous_codec::Framed;
use bytes::Bytes;
use libp2p::swarm::handler::{
    ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
};
use libp2p::swarm::{
    ConnectionHandler, ConnectionHandlerEvent, KeepAlive, StreamUpgradeError, SubstreamProtocol,
};

use common::service::{BufferedContext, ServiceContext};

use crate::conn_handler::events::{StreamHandlerIn, StreamHandlerOut};
use crate::conn_handler::service_downstream::DownstreamHandler;
use crate::conn_handler::service_upstream::UpstreamHandler;
use crate::upgrade::{ProtocolUpgradeOutput, SimpleProtocolUpgrade};

use super::codec::Codec;
use super::events::{Command, Event};

type Upgrade = SimpleProtocolUpgrade<&'static str>;
type UpgradeOutput = ProtocolUpgradeOutput<&'static str>;

/// A connection handler that manages a single, inbound and outbound, long-lived substream over
/// a connection with a peer.
pub struct Handler {
    /// Upgrade configuration for the protocol.
    upgrade: Upgrade,

    /// Maximum frame size.
    max_frame_size: usize,

    /// Queue of values that we want to send to the remote.
    outbound_queue: VecDeque<Bytes>,

    /// The single long-lived outbound substream.
    outbound_substream: BufferedContext<DownstreamHandler>,

    /// The single long-lived inbound substream.
    inbound_substream: BufferedContext<UpstreamHandler>,

    /// Flag indicating that an outbound substream is being established to prevent duplicate
    /// requests.
    outbound_substream_establishing: bool,

    /// The last time we performed IO on the connection.
    last_io_activity: Instant,

    /// The amount of time we keep an idle connection alive.
    idle_timeout: Duration,
}

impl Handler {
    pub fn new(upgrade: Upgrade, max_frame_size: usize, idle_timeout: Duration) -> Self {
        Self {
            upgrade,
            max_frame_size,
            outbound_substream: Default::default(),
            inbound_substream: Default::default(),
            outbound_substream_establishing: false,
            outbound_queue: Default::default(),
            last_io_activity: Instant::now(),
            idle_timeout,
        }
    }

    fn on_fully_negotiated_inbound(&mut self, output: UpgradeOutput) {
        let UpgradeOutput { socket, .. } = output;

        let codec = Codec::new(self.max_frame_size);
        let stream = Framed::new(socket, codec);

        tracing::trace!("New fully negotiated inbound substream");

        // The substream is fully negotiated. Initialize the substream handler.
        self.inbound_substream
            .do_send(StreamHandlerIn::Init(stream));
    }

    fn on_fully_negotiated_outbound(&mut self, output: UpgradeOutput) {
        let UpgradeOutput { socket, .. } = output;

        let codec = Codec::new(self.max_frame_size);
        let stream = Framed::new(socket, codec);

        tracing::trace!("New fully negotiated outbound substream");

        // The substream is fully negotiated. Initialize the substream handler.
        self.outbound_substream
            .do_send(StreamHandlerIn::Init(stream));

        // Send all queued messages into the outbound substream.
        for frame in self.outbound_queue.drain(..) {
            self.outbound_substream
                .do_send(StreamHandlerIn::SendFrame(frame));
        }
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = Command;
    type ToBehaviour = Event;
    type Error = Infallible;
    type InboundProtocol = Upgrade;
    type OutboundProtocol = Upgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(self.upgrade.clone(), ())
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        if self.outbound_substream.is_sending() {
            return KeepAlive::Yes;
        }

        KeepAlive::Until(self.last_io_activity + self.idle_timeout)
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
            Self::Error,
        >,
    > {
        // determine if we need to create the outbound stream
        if !self.outbound_queue.is_empty()
            && self.outbound_substream.is_disabled()
            && !self.outbound_substream_establishing
        {
            self.outbound_substream_establishing = true;

            tracing::trace!("new outbound substream request");

            // Send a request to open a new outbound substream.
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(self.upgrade.clone(), ()),
            });
        }

        // Poll the inbound substream (upstream).
        if let Poll::Ready(ev) = self.inbound_substream.poll(cx) {
            match ev {
                StreamHandlerOut::FrameReceived(bytes) => {
                    // Update the last IO activity time.
                    self.last_io_activity = Instant::now();

                    // Notify the behaviour about the received frame.
                    return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                        Event::FrameReceived(bytes),
                    ));
                }
            }
        }

        // Poll outbound stream (downstream).
        let _ = self.outbound_substream.poll(cx);

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        tracing::trace!(?event, "Received behaviour event");
        match event {
            Command::SendFrame(msg) => {
                // If the outbound substream is disabled, queue the message. Otherwise, send it
                // into the outbound substream handler mailbox.
                if self.outbound_substream.is_disabled() {
                    self.outbound_queue.push_back(msg);
                } else {
                    // Update the last IO activity time.
                    self.last_io_activity = Instant::now();

                    // Notify the outbound substream handler about the message.
                    self.outbound_substream
                        .do_send(StreamHandlerIn::SendFrame(msg))
                }
            }
        }
    }

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        if event.is_outbound() {
            self.outbound_substream_establishing = false;
        }

        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol, ..
            }) => {
                self.on_fully_negotiated_inbound(protocol);
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol, ..
            }) => {
                self.on_fully_negotiated_outbound(protocol);
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError {
                error: StreamUpgradeError::Timeout,
                ..
            }) => {
                tracing::debug!("Dial upgrade error: Protocol negotiation timeout");
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError {
                error: StreamUpgradeError::Io(e),
                ..
            }) => {
                tracing::debug!("Protocol negotiation failed: {e}")
            }
            _ => {}
        }
    }
}
