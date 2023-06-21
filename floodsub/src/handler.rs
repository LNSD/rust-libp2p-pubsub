use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use asynchronous_codec::Framed;
use futures::future::Either;
use futures::prelude::*;
use libp2p::core::upgrade::DeniedUpgrade;
use libp2p::swarm::handler::{
    ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
};
use libp2p::swarm::{
    ConnectionHandler, ConnectionHandlerEvent, KeepAlive, Stream, StreamUpgradeError,
    SubstreamProtocol,
};
use smallvec::SmallVec;

use common::prost_protobuf_codec::Codec as ProstCodec;

use crate::proto::RpcProto;
use crate::protocol_id::StaticProtocolId;
use crate::upgrade::ProtocolUpgrade;

type Codec = ProstCodec<RpcProto>;
type ProtocolId = StaticProtocolId<&'static str>;

#[derive(Debug)]
pub enum Command {
    /// A RPC frame to send.
    SendFrame(RpcProto),

    /// Keep the connection alive.
    KeepAlive,
}

#[derive(Debug)]
pub enum Event {
    /// A pubsub message has been received.
    FrameReceived(RpcProto),

    /// The handler has been disabled.
    Disabled(DisabledHandlerReason),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, strum_macros::Display)]
pub enum DisabledHandlerReason {
    /// If the peer doesn't support the gossipsub protocol we do not immediately disconnect.
    /// Rather, we disable the handler and prevent any incoming or outgoing substreams from being
    /// established.
    ProtocolUnsupported,
    /// The maximum number of inbound or outbound substream attempts have happened and thereby the
    /// handler has been disabled.
    MaxSubstreamAttempts,
}

#[derive(Debug)]
pub struct DisabledHandler {
    /// The reason why the handler was disabled.
    reason: DisabledHandlerReason,

    /// Reason was emitted to the user.
    reason_emitted: bool,
}

impl DisabledHandler {
    pub fn with_reason(reason: DisabledHandlerReason) -> Self {
        Self {
            reason,
            reason_emitted: false,
        }
    }
}

impl ConnectionHandler for DisabledHandler {
    type FromBehaviour = Command;
    type ToBehaviour = Event;
    type Error = Infallible;
    type InboundProtocol = either::Either<ProtocolUpgrade, DeniedUpgrade>;
    type OutboundProtocol = ProtocolUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    /// This method acts as a factory method for [`InboundUpgrade`] to apply on inbound
    /// substreams to negotiate the desired protocols.
    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(either::Either::Right(DeniedUpgrade), ())
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::No
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
            Self::Error,
        >,
    > {
        if !self.reason_emitted {
            self.reason_emitted = true;
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(Event::Disabled(
                self.reason,
            )));
        }

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        log::debug!("Ignoring incoming message because handler is disabled: {event:?}")
    }

    fn on_connection_event(
        &mut self,
        _event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        // No-op
    }
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
    PendingSend(Framed<Stream, Codec>, RpcProto),
    /// Waiting to flush the substream so that the data arrives to the remote.
    PendingFlush(Framed<Stream, Codec>),
    /// An error occurred during processing.
    Poisoned,
}

/// A connection handler that manages a single, inbound and outbound, long-lived substream over
/// a connection with a peer.
pub struct SimpleHandler {
    /// Upgrade configuration for the protocol.
    upgrade: ProtocolUpgrade,

    /// The single long-lived outbound substream.
    outbound_substream: Option<OutboundSubstreamState>,

    /// The single long-lived inbound substream.
    inbound_substream: Option<InboundSubstreamState>,

    /// Queue of values that we want to send to the remote.
    send_queue: SmallVec<[RpcProto; 16]>,

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

impl SimpleHandler {
    pub(crate) fn new(listen_protocol: ProtocolUpgrade, idle_timeout: Duration) -> Self {
        Self {
            upgrade: listen_protocol,
            outbound_substream: None,
            inbound_substream: None,
            send_queue: SmallVec::new(),
            outbound_substream_establishing: false,
            last_io_activity: Instant::now(),
            idle_timeout,
            keep_alive: false,
        }
    }

    fn on_fully_negotiated_inbound(
        &mut self,
        (substream, _protocol_id): (Framed<Stream, Codec>, ProtocolId),
    ) {
        // new inbound substream. Replace the current one, if it exists.
        log::trace!("new inbound substream request");
        self.inbound_substream = Some(InboundSubstreamState::WaitingInput(substream));
    }

    fn on_fully_negotiated_outbound(
        &mut self,
        FullyNegotiatedOutbound { protocol, .. }: FullyNegotiatedOutbound<
            <Self as ConnectionHandler>::OutboundProtocol,
            <Self as ConnectionHandler>::OutboundOpenInfo,
        >,
    ) {
        let (substream, _protocol_id) = protocol;

        assert!(
            self.outbound_substream.is_none(),
            "Established an outbound substream with one already available"
        );
        self.outbound_substream = Some(OutboundSubstreamState::WaitingOutput(substream));
    }
}

impl ConnectionHandler for SimpleHandler {
    type FromBehaviour = Command;
    type ToBehaviour = Event;
    type Error = Infallible;
    type InboundProtocol = either::Either<ProtocolUpgrade, DeniedUpgrade>;
    type OutboundProtocol = ProtocolUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(either::Either::Left(self.upgrade.clone()), ())
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
                            log::debug!("Failed to read from inbound stream: {error}");
                            // Close this side of the stream. If the
                            // peer is still around, they will re-establish their
                            // outbound stream i.e. our inbound stream.
                            self.inbound_substream =
                                Some(InboundSubstreamState::Closing(substream));
                        }
                        // peer closed the stream
                        Poll::Ready(None) => {
                            log::debug!("Inbound stream closed by remote");
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
                                log::debug!("Inbound substream error while closing: {e}");
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
                Some(OutboundSubstreamState::PendingSend(mut substream, message)) => {
                    match Sink::poll_ready(Pin::new(&mut substream), cx) {
                        Poll::Ready(Ok(())) => {
                            match Sink::start_send(Pin::new(&mut substream), message) {
                                Ok(()) => {
                                    self.outbound_substream =
                                        Some(OutboundSubstreamState::PendingFlush(substream))
                                }
                                Err(e) => {
                                    log::debug!("Failed to send message on outbound stream: {e}");
                                    self.outbound_substream = None;
                                    break;
                                }
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            log::debug!("Failed to send message on outbound stream: {e}");
                            self.outbound_substream = None;
                            break;
                        }
                        Poll::Pending => {
                            self.outbound_substream =
                                Some(OutboundSubstreamState::PendingSend(substream, message));
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
                            log::debug!("Failed to flush outbound stream: {e}");
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
            }) => match protocol {
                Either::Left(protocol) => {
                    self.on_fully_negotiated_inbound(protocol);
                }
                Either::Right(_) => unreachable!(),
            },
            ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated_outbound) => {
                self.on_fully_negotiated_outbound(fully_negotiated_outbound)
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError {
                error: StreamUpgradeError::Timeout,
                ..
            }) => {
                log::debug!("Dial upgrade error: Protocol negotiation timeout");
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError {
                error: StreamUpgradeError::Io(e),
                ..
            }) => {
                log::debug!("Protocol negotiation failed: {e}")
            }
            _ => {}
        }
    }
}

/// The maximum number of inbound or outbound substreams attempts we allow.
///
/// Floodsub is supposed to have a single long-lived inbound and outbound substream. On failure we
/// attempt to recreate these. This imposes an upper bound of new substreams before we consider the
/// connection faulty and disable the handler. This also prevents against potential substream
/// creation loops.
const MAX_SUBSTREAM_ATTEMPTS: usize = 5;

pub enum HandlerState {
    /// The handler is enabled and can send and receive messages.
    Enabled(SimpleHandler),
    /// The handler is disabled and cannot send or receive messages.
    Disabled(DisabledHandler),
}

pub struct Handler {
    /// The number of inbound substreams that have been created by the peer.
    inbound_substream_attempts: usize,

    /// The number of outbound substreams we have requested.
    outbound_substream_attempts: usize,

    /// The state of the handler.
    inner: HandlerState,
}

impl Handler {
    // TODO: Make generic, decouple from Frame frame, from ProtocolUpgrade, etc.
    pub fn new(max_frame_size: usize, idle_timeout: Duration) -> Self {
        let upgrade = ProtocolUpgrade::new(max_frame_size);
        Self {
            inbound_substream_attempts: 0,
            outbound_substream_attempts: 0,
            inner: HandlerState::Enabled(SimpleHandler::new(upgrade, idle_timeout)),
        }
    }

    #[cfg(test)]
    pub fn is_enabled(&self) -> bool {
        matches!(self.inner, HandlerState::Enabled(_))
    }

    #[cfg(test)]
    pub fn is_disabled(&self) -> bool {
        matches!(self.inner, HandlerState::Disabled(_))
    }

    #[cfg(test)]
    pub fn disable(&mut self, reason: DisabledHandlerReason) {
        self.inner = HandlerState::Disabled(DisabledHandler::with_reason(reason));
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = Command;
    type ToBehaviour = Event;
    type Error = Infallible;
    type InboundProtocol = either::Either<ProtocolUpgrade, DeniedUpgrade>;
    type OutboundProtocol = ProtocolUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        match &self.inner {
            HandlerState::Enabled(handler) => handler.listen_protocol(),
            HandlerState::Disabled(handler) => handler.listen_protocol(),
        }
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        match &self.inner {
            HandlerState::Enabled(handler) => handler.connection_keep_alive(),
            HandlerState::Disabled(handler) => handler.connection_keep_alive(),
        }
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
        match &mut self.inner {
            HandlerState::Enabled(handler) => handler.poll(cx),
            HandlerState::Disabled(handler) => handler.poll(cx),
        }
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        match &mut self.inner {
            HandlerState::Enabled(handler) => handler.on_behaviour_event(event),
            HandlerState::Disabled(handler) => handler.on_behaviour_event(event),
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
        if event.is_inbound() {
            self.inbound_substream_attempts += 1;

            if self.inbound_substream_attempts >= MAX_SUBSTREAM_ATTEMPTS {
                log::warn!("The maximum number of inbound substreams attempts has been exceeded");
                self.inner = HandlerState::Disabled(DisabledHandler::with_reason(
                    DisabledHandlerReason::MaxSubstreamAttempts,
                ));
                return;
            }
        }

        if event.is_outbound() {
            self.outbound_substream_attempts += 1;

            if self.outbound_substream_attempts >= MAX_SUBSTREAM_ATTEMPTS {
                log::warn!("The maximum number of outbound substream attempts has been exceeded");
                self.inner = HandlerState::Disabled(DisabledHandler::with_reason(
                    DisabledHandlerReason::MaxSubstreamAttempts,
                ));
                return;
            }
        }

        if let ConnectionEvent::DialUpgradeError(DialUpgradeError {
            error: StreamUpgradeError::NegotiationFailed,
            ..
        }) = event
        {
            // The protocol is not supported
            log::debug!("The remote peer does not support the protocol on this connection");
            self.inner = HandlerState::Disabled(DisabledHandler::with_reason(
                DisabledHandlerReason::ProtocolUnsupported,
            ));
        }

        match &mut self.inner {
            HandlerState::Enabled(handler) => handler.on_connection_event(event),
            HandlerState::Disabled(handler) => handler.on_connection_event(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use libp2p::core::UpgradeInfo;

    use super::*;

    fn test_handler() -> Handler {
        Handler::new(1024, Duration::from_secs(60))
    }

    #[test]
    fn handler_is_enabled_by_default() {
        //// Given
        let handler = test_handler();

        //// Then
        assert!(handler.is_enabled());
    }

    #[test]
    fn handler_disables_after_max_attempts_inbound() {
        //// Given
        let mut handler = test_handler();

        //// When
        for _ in 0..MAX_SUBSTREAM_ATTEMPTS {
            handler.on_connection_event(ConnectionEvent::DialUpgradeError(DialUpgradeError {
                info: (),
                error: StreamUpgradeError::NegotiationFailed,
            }));
        }

        //// Then
        assert!(handler.is_disabled());
    }

    #[test]
    fn handler_disables_after_max_attempts_outbound() {
        //// Given
        let mut handler = test_handler();

        //// When
        for _ in 0..MAX_SUBSTREAM_ATTEMPTS {
            handler.on_connection_event(ConnectionEvent::DialUpgradeError(DialUpgradeError {
                info: (),
                error: StreamUpgradeError::NegotiationFailed,
            }));
        }

        //// Then
        assert!(handler.is_disabled());
    }

    #[test]
    fn enabled_handler_listen_protocol() {
        //// Given
        let handler = test_handler();

        //// When
        let protocol = handler.listen_protocol();

        //// Then
        assert_matches!(
            protocol
                .upgrade()
                .as_ref()
                .left()
                .map(|p| p.protocol_info()),
            Some(p) => {
                assert_eq!(p.len(), 1);
            }
        );
    }

    #[test]
    fn disabled_handler_listen_protocol() {
        //// Given
        let mut handler = test_handler();
        handler.disable(DisabledHandlerReason::ProtocolUnsupported);

        //// When
        let protocol = handler.listen_protocol();

        //// Then
        assert_matches!(
            protocol
                .upgrade()
                .as_ref()
                .right()
                .map(|p| p.protocol_info()),
            Some(p) => {
                assert_eq!(p.len(), 0);
            }
        );
    }

    #[test]
    fn enabled_handler_connection_keep_alive() {
        //// Given
        let handler = test_handler();

        //// When
        let keep_alive = handler.connection_keep_alive();

        //// Then
        assert_matches!(keep_alive, KeepAlive::Until(_));
    }

    #[test]
    fn disabled_handler_connection_keep_alive() {
        //// Given
        let mut handler = test_handler();
        handler.disable(DisabledHandlerReason::ProtocolUnsupported);

        //// When
        let keep_alive = handler.connection_keep_alive();

        //// Then
        assert_eq!(keep_alive, KeepAlive::No);
    }

    #[test]
    fn enabled_handler_on_keep_alive_command() {
        //// Given
        let mut handler = test_handler();

        //// When
        handler.on_behaviour_event(Command::KeepAlive);

        //// Then
        let keep_alive = handler.connection_keep_alive();
        assert_matches!(keep_alive, KeepAlive::Yes);
    }
}
