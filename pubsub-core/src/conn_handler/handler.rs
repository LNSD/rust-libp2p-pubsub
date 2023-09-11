use std::convert::Infallible;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use asynchronous_codec::Framed;
use libp2p::swarm::handler::{
    ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
};
use libp2p::swarm::{
    ConnectionHandler, ConnectionHandlerEvent, KeepAlive, StreamUpgradeError, SubstreamProtocol,
};

use libp2p_pubsub_common::service::{BufferedContext, ServiceContext};

use crate::conn_handler::downstream::{
    DownstreamConnHandlerInEvent, DownstreamConnHandlerOutEvent, DownstreamIn, DownstreamOut,
};
use crate::upgrade::{ProtocolUpgradeOutput, ProtocolUpgradeSend};

use super::codec::Codec;
use super::downstream::Downstream;
use super::events::{Command, Event};
use super::events_stream_handler::StreamHandlerOut;
use super::recv_only_stream_handler::RecvOnlyStreamHandler;

/// A connection handler that manages a single, inbound and outbound, long-lived substream over
/// a connection with a peer.
pub struct Handler<U> {
    /// The protocol upgrade.
    upgrade: U,

    /// A flag indicating if the connection should be kept alive.
    ///
    /// If the outbound substream has reached the maximum number of send retries, or the upgrade
    /// failed, the connection is marked as not keep alive and will be closed.
    keep_alive: bool,

    /// Maximum frame size.
    max_frame_size: usize,

    /// The single long-lived outbound substream.
    downstream: BufferedContext<Downstream>,

    /// The single long-lived inbound substream.
    inbound_substream: Option<BufferedContext<RecvOnlyStreamHandler>>,

    /// The last time we performed IO on the connection.
    last_io_activity: Instant,

    /// The amount of time we keep an idle connection alive.
    idle_timeout: Duration,
}

impl<U> Handler<U>
where
    U: ProtocolUpgradeSend + 'static,
{
    pub fn new(
        upgrade: U,
        max_frame_size: usize,
        idle_timeout: Duration,
        max_send_retry_attempts: usize,
    ) -> Self {
        Self {
            upgrade,
            keep_alive: true,
            max_frame_size,
            downstream: BufferedContext::new(Downstream::new(max_send_retry_attempts)),
            inbound_substream: Default::default(),
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
        if self.downstream.is_sending() {
            return KeepAlive::Yes;
        }

        if self.keep_alive {
            return KeepAlive::Until(self.last_io_activity + self.idle_timeout);
        }

        KeepAlive::No
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
        // If the connection is marked as not keep alive, do nothing.
        if !self.keep_alive {
            return Poll::Pending;
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

        // Poll the downstream handler (outbound).
        if let Poll::Ready(ev) = self.downstream.poll(cx) {
            match ev {
                Ok(DownstreamOut::SendAck) => {
                    // Update the last IO activity time.
                    self.last_io_activity = Instant::now();

                    // Notify the behaviour about the received frame.
                    return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(Event::FrameSent));
                }
                Ok(DownstreamOut::ConnHandlerEvent(
                    DownstreamConnHandlerOutEvent::RequestNewSubstream,
                )) => {
                    // Request a new outbound substream.
                    return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                        protocol: SubstreamProtocol::new(self.upgrade.clone(), ()),
                    });
                }
                Err(err) => {
                    tracing::debug!("Downstream handler error: {:?}", err);

                    // Mark the connection as not keep alive.
                    self.keep_alive = false;
                }
            }
        }

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        tracing::trace!(?event, "Received behaviour event");
        match event {
            Command::SendFrame(bytes) => {
                // Notify the downstream handler about the new frame to be sent.
                self.downstream.do_send(DownstreamIn::Send(bytes));
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
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol, ..
            }) => {
                let ProtocolUpgradeOutput { socket, .. } = protocol;

                let codec = Codec::new(self.max_frame_size);
                let stream = Framed::new(socket, codec);

                tracing::trace!("New fully negotiated inbound substream");

                // The substream is fully negotiated. Initialize the substream handler.
                self.inbound_substream =
                    Some(BufferedContext::new(RecvOnlyStreamHandler::new(stream)));
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol, ..
            }) => {
                let ProtocolUpgradeOutput { socket, .. } = protocol;

                let codec = Codec::new(self.max_frame_size);
                let stream = Framed::new(socket, codec);

                tracing::trace!("New fully negotiated outbound substream");

                // Initialize the downstream handler with the new outbound substream.
                self.downstream.do_send(DownstreamIn::ConnHandlerEvent(
                    DownstreamConnHandlerInEvent::FullyNegotiated(stream),
                ));
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError {
                error: StreamUpgradeError::Timeout,
                ..
            }) => {
                tracing::debug!("Dial upgrade error: Protocol negotiation timeout");
                self.downstream.do_send(DownstreamIn::ConnHandlerEvent(
                    DownstreamConnHandlerInEvent::UpradeError,
                ));
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError {
                error: StreamUpgradeError::Io(e),
                ..
            }) => {
                tracing::debug!("Protocol negotiation failed: {e}");
                self.downstream.do_send(DownstreamIn::ConnHandlerEvent(
                    DownstreamConnHandlerInEvent::UpradeError,
                ));
            }
            _ => {}
        }
    }
}
