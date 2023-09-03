use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use asynchronous_codec::Framed;
use bytes::Bytes;
use futures::{Sink, StreamExt};
use libp2p::swarm::handler::{
    ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
};
use libp2p::swarm::{
    ConnectionHandler, ConnectionHandlerEvent, KeepAlive, Stream, StreamUpgradeError,
    SubstreamProtocol,
};
use smallvec::SmallVec;

use common::upgrade::{SimpleUpgrade, SimpleUpgradeOutput};

use super::codec::Codec;

type ProtocolId = &'static str;
type Upgrade = SimpleUpgrade<ProtocolId>;
type UpgradeOutput = SimpleUpgradeOutput<ProtocolId, Stream>;

#[derive(Debug)]
pub enum Command {
    /// A pubsub frame to send to the remote.
    SendFrame(Bytes),

    /// Keep the connection alive.
    KeepAlive,
}

#[derive(Debug)]
pub enum Event {
    /// A pubsub frame has been received.
    FrameReceived(Bytes),
}

enum InboundSubstreamState {
    /// Waiting for a message from the remote. The idle state for an inbound substream.
    WaitingInput(Framed<Stream, Codec>),
    /// The substream is being closed.
    Closing(Framed<Stream, Codec>),
    /// An error occurred during processing.
    Poisoned,
}

/// State of the outbound substream, opened either by us or by the remote.
enum OutboundSubstreamState {
    /// Waiting for the user to send a message. The idle state for an outbound substream.
    WaitingOutput(Framed<Stream, Codec>),
    /// Waiting to send a message to the remote.
    PendingSend(Framed<Stream, Codec>, Bytes),
    /// Waiting to flush the substream so that the data arrives to the remote.
    PendingFlush(Framed<Stream, Codec>),
    /// An error occurred during processing.
    Poisoned,
}

/// A connection handler that manages a single, inbound and outbound, long-lived substream over
/// a connection with a peer.
pub struct Handler {
    /// Upgrade configuration for the protocol.
    upgrade: Upgrade,

    /// Maximum frame size.
    max_frame_size: usize,

    /// The single long-lived outbound substream.
    outbound_substream: Option<OutboundSubstreamState>,

    /// The single long-lived inbound substream.
    inbound_substream: Option<InboundSubstreamState>,

    /// Queue of values that we want to send to the remote.
    send_queue: SmallVec<[Bytes; 16]>,

    /// Flag indicating that an outbound substream is being established to prevent duplicate
    /// requests.
    outbound_substream_establishing: bool,

    /// The last time we performed IO on the connection.
    last_io_activity: Instant,

    /// The amount of time we keep an idle connection alive.
    idle_timeout: Duration,

    /// Keep connection alive.
    keep_alive: bool,
}

impl Handler {
    pub fn new(protocol_id: ProtocolId, max_frame_size: usize, idle_timeout: Duration) -> Self {
        let upgrade = Upgrade::new(protocol_id);
        Self {
            upgrade,
            max_frame_size,
            outbound_substream: None,
            inbound_substream: None,
            send_queue: SmallVec::new(),
            outbound_substream_establishing: false,
            last_io_activity: Instant::now(),
            idle_timeout,
            keep_alive: false,
        }
    }

    fn on_fully_negotiated_inbound(&mut self, protocol: UpgradeOutput) {
        let UpgradeOutput { socket, .. } = protocol;

        let codec = Codec::new(self.max_frame_size);
        let stream = Framed::new(socket, codec);

        // new inbound substream. Replace the current one, if it exists.
        tracing::trace!("new inbound substream request");
        self.inbound_substream = Some(InboundSubstreamState::WaitingInput(stream));
    }

    fn on_fully_negotiated_outbound(
        &mut self,
        FullyNegotiatedOutbound { protocol, .. }: FullyNegotiatedOutbound<
            <Self as ConnectionHandler>::OutboundProtocol,
            <Self as ConnectionHandler>::OutboundOpenInfo,
        >,
    ) {
        let UpgradeOutput { socket, .. } = protocol;

        let codec = Codec::new(self.max_frame_size);
        let stream = Framed::new(socket, codec);

        assert!(
            self.outbound_substream.is_none(),
            "Established an outbound substream with one already available"
        );
        self.outbound_substream = Some(OutboundSubstreamState::WaitingOutput(stream));
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
        if self.keep_alive {
            return KeepAlive::Yes;
        }

        if matches!(
            self.outbound_substream,
            Some(
                OutboundSubstreamState::PendingSend(_, _) | OutboundSubstreamState::PendingFlush(_),
            )
        ) {
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
        if !self.send_queue.is_empty()
            && self.outbound_substream.is_none()
            && !self.outbound_substream_establishing
        {
            self.outbound_substream_establishing = true;

            // Send a request to open a new outbound substream.
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(self.upgrade.clone(), ()),
            });
        }

        // Process the inbound substream.
        loop {
            match std::mem::replace(
                &mut self.inbound_substream,
                Some(InboundSubstreamState::Poisoned),
            ) {
                // inbound idle state
                Some(InboundSubstreamState::WaitingInput(mut substream)) => {
                    match substream.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(message))) => {
                            self.last_io_activity = Instant::now();
                            self.inbound_substream =
                                Some(InboundSubstreamState::WaitingInput(substream));
                            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                                Event::FrameReceived(message),
                            ));
                        }
                        Poll::Ready(Some(Err(error))) => {
                            tracing::debug!("Failed to read from inbound stream: {error}");
                            // Close this side of the stream. If the
                            // peer is still around, they will re-establish their
                            // outbound stream i.e. our inbound stream.
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                        }
                        // peer closed the stream
                        Poll::Ready(None) => {
                            tracing::debug!("Inbound stream closed by remote");
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                        }
                        Poll::Pending => {
                            self.inbound_substream =
                                Some(InboundSubstreamState::WaitingInput(substream));
                            break;
                        }
                    }
                }
                Some(InboundSubstreamState::Closing(mut substream)) => {
                    match Sink::poll_close(Pin::new(&mut substream), cx) {
                        Poll::Ready(res) => {
                            if let Err(e) = res {
                                // Don't close the connection but just drop the inbound substream.
                                // In case the remote has more to send, they will open up a new
                                // substream.
                                tracing::debug!("Inbound substream error while closing: {e}");
                            }
                            self.inbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                            break;
                        }
                    }
                }
                None => {
                    self.inbound_substream = None;
                    break;
                }
                Some(InboundSubstreamState::Poisoned) => {
                    unreachable!("Error occurred during inbound stream processing")
                }
            }
        }

        // Process outbound stream.
        loop {
            match std::mem::replace(
                &mut self.outbound_substream,
                Some(OutboundSubstreamState::Poisoned),
            ) {
                // outbound idle state
                Some(OutboundSubstreamState::WaitingOutput(substream)) => {
                    if let Some(message) = self.send_queue.pop() {
                        self.send_queue.shrink_to_fit();
                        self.outbound_substream =
                            Some(OutboundSubstreamState::PendingSend(substream, message));
                        continue;
                    }

                    self.outbound_substream =
                        Some(OutboundSubstreamState::WaitingOutput(substream));
                    break;
                }
                Some(OutboundSubstreamState::PendingSend(mut substream, bytes)) => {
                    match Sink::poll_ready(Pin::new(&mut substream), cx) {
                        Poll::Ready(Ok(())) => {
                            match Sink::start_send(Pin::new(&mut substream), bytes) {
                                Ok(()) => {
                                    self.outbound_substream =
                                        Some(OutboundSubstreamState::PendingFlush(substream))
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Failed to send message on outbound stream: {e}"
                                    );
                                    self.outbound_substream = None;
                                    break;
                                }
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            tracing::debug!("Failed to send message on outbound stream: {e}");
                            self.outbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.outbound_substream =
                                Some(OutboundSubstreamState::PendingSend(substream, bytes));
                            break;
                        }
                    }
                }
                Some(OutboundSubstreamState::PendingFlush(mut substream)) => {
                    match Sink::poll_flush(Pin::new(&mut substream), cx) {
                        Poll::Ready(Ok(())) => {
                            self.last_io_activity = Instant::now();
                            self.outbound_substream =
                                Some(OutboundSubstreamState::WaitingOutput(substream))
                        }
                        Poll::Ready(Err(e)) => {
                            tracing::debug!("Failed to flush outbound stream: {e}");
                            self.outbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.outbound_substream =
                                Some(OutboundSubstreamState::PendingFlush(substream));
                            break;
                        }
                    }
                }
                None => {
                    self.outbound_substream = None;
                    break;
                }
                Some(OutboundSubstreamState::Poisoned) => {
                    unreachable!("Error occurred during outbound stream processing")
                }
            }
        }

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        tracing::trace!(?event, "Received behaviour event");
        match event {
            Command::SendFrame(msg) => self.send_queue.push(msg),
            Command::KeepAlive => {
                self.keep_alive = true;
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
            ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated_outbound) => {
                self.on_fully_negotiated_outbound(fully_negotiated_outbound)
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
