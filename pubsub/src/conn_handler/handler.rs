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
use crate::upgrade::{ProtocolUpgradeOutput, ProtocolUpgradeSend};

use super::codec::Codec;
use super::events::{Command, Event};

/// A connection handler that manages a single, inbound and outbound, long-lived substream over
/// a connection with a peer.
pub struct Handler<U> {
    /// The protocol upgrade.
    upgrade: U,

    /// Maximum frame size.
    max_frame_size: usize,

    /// Queue of values that we want to send to the remote.
    send_queue: VecDeque<Bytes>,

    /// The single long-lived outbound substream.
    outbound_substream: Option<BufferedContext<DownstreamHandler>>,

    /// The single long-lived inbound substream.
    inbound_substream: Option<BufferedContext<UpstreamHandler>>,

    /// Flag indicating that an outbound substream is being established to prevent duplicate
    /// requests.
    outbound_substream_establishing: bool,

    /// The last time we performed IO on the connection.
    last_io_activity: Instant,

    /// The amount of time we keep an idle connection alive.
    idle_timeout: Duration,
}

impl<U> Handler<U>
where
    U: ProtocolUpgradeSend + 'static,
{
    pub fn new(upgrade: U, max_frame_size: usize, idle_timeout: Duration) -> Self {
        Self {
            upgrade,
            max_frame_size,
            outbound_substream: Default::default(),
            inbound_substream: Default::default(),
            outbound_substream_establishing: false,
            send_queue: Default::default(),
            last_io_activity: Instant::now(),
            idle_timeout,
        }
    }
}

impl<U> ConnectionHandler for Handler<U>
where
    U: ProtocolUpgradeSend + Clone,
{
    type FromBehaviour = Command;
    type ToBehaviour = Event;
    type Error = Infallible;
    type InboundProtocol = U;
    type OutboundProtocol = U;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(self.upgrade.clone(), ())
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        if let Some(outbound_substream) = self.outbound_substream.as_ref() {
            if outbound_substream.is_sending() {
                return KeepAlive::Yes;
            }
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
        if !self.send_queue.is_empty()
            && self.outbound_substream.is_none()
            && !self.outbound_substream_establishing
        {
            self.outbound_substream_establishing = true;

            tracing::trace!("new outbound substream request");

            // Send a request to open a new outbound substream.
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(self.upgrade.clone(), ()),
            });
        }

        if let Some(mut inbound_substream) = self.inbound_substream.take() {
            // Poll the inbound substream (upstream).
            if let Poll::Ready(ev) = inbound_substream.poll(cx) {
                match ev {
                    Ok(StreamHandlerOut::FrameReceived(bytes)) => {
                        // Update the last IO activity time.
                        self.last_io_activity = Instant::now();

                        // Re-insert the substream into the handler.
                        self.inbound_substream = Some(inbound_substream);

                        // Notify the behaviour about the received frame.
                        return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                            Event::FrameReceived(bytes),
                        ));
                    }
                    Err(err) => {
                        tracing::debug!("Inbound substream error: {}", err);

                        // Drop the inbound substream.
                        self.inbound_substream = None;
                    }
                    _ => {
                        unreachable!("unexpected event: {:?}", ev);
                    }
                }
            } else {
                // Re-insert the substream into the handler.
                self.inbound_substream = Some(inbound_substream);
            }
        }

        if let Some(mut outbound_substream) = self.outbound_substream.take() {
            // If the outbound substream is idle, try to send the next message from the send
            // queue.
            if outbound_substream.is_idle() {
                if let Some(msg) = self.send_queue.pop_front() {
                    // Notify the outbound substream handler about the message.
                    outbound_substream.do_send(StreamHandlerIn::SendFrame(msg));
                }
            }

            // Poll outbound stream (downstream).
            if let Poll::Ready(ev) = outbound_substream.poll(cx) {
                match ev {
                    Ok(StreamHandlerOut::FrameSent) => {
                        // Update the last IO activity time.
                        self.last_io_activity = Instant::now();

                        // Re-insert the substream into the handler.
                        self.outbound_substream = Some(outbound_substream);

                        // Notify the behaviour about the sent frame.
                        return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                            Event::FrameSent,
                        ));
                    }
                    Err(err) => {
                        tracing::debug!("Outbound substream error: {}", err);

                        // Drop the outbound substream.
                        self.outbound_substream = None;
                    }
                    _ => {
                        unreachable!("unexpected event: {:?}", ev);
                    }
                }
            } else {
                // Re-insert the substream into the handler.
                self.outbound_substream = Some(outbound_substream);
            }
        }

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        tracing::trace!(?event, "Received behaviour event");
        match event {
            Command::SendFrame(msg) => {
                // Add the message to the send queue.
                self.send_queue.push_back(msg);
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
                let ProtocolUpgradeOutput { socket, .. } = protocol;

                let codec = Codec::new(self.max_frame_size);
                let stream = Framed::new(socket, codec);

                tracing::trace!("New fully negotiated inbound substream");

                // The substream is fully negotiated. Initialize the substream handler.
                self.inbound_substream = Some(BufferedContext::new(UpstreamHandler::new(stream)));
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol, ..
            }) => {
                let ProtocolUpgradeOutput { socket, .. } = protocol;

                let codec = Codec::new(self.max_frame_size);
                let stream = Framed::new(socket, codec);

                tracing::trace!("New fully negotiated outbound substream");

                // The substream is fully negotiated. Initialize the substream handler.
                self.outbound_substream =
                    Some(BufferedContext::new(DownstreamHandler::new(stream)));
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
