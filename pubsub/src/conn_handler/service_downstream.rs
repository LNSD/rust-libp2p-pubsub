use std::pin::Pin;
use std::task::{Context, Poll};

use asynchronous_codec::Framed;
use bytes::Bytes;
use futures::Sink;
use libp2p::swarm::Stream;

use common::service::{PollCtx, Service};

use super::codec::Codec;
use super::events::StreamHandlerIn;

/// State of the outbound substream, opened either by us or by the remote.
#[derive(Default)]
enum SubstreamState {
    /// Disabled state.
    #[default]
    Disabled,
    /// Waiting for the user to send a message. The idle state for an outbound substream.
    Idle(Framed<Stream, Codec>),
    /// Waiting to send a message to the remote.
    PendingSend(Framed<Stream, Codec>, Bytes),
    /// Waiting to flush the substream so that the data arrives to the remote.
    PendingFlush(Framed<Stream, Codec>),
    /// An error occurred during processing.
    Poisoned,
}

#[derive(Default)]
pub struct DownstreamHandler {
    state: SubstreamState,
}

impl DownstreamHandler {
    /// Returns `true` if the substream is in the process of sending a message.
    pub fn is_sending(&self) -> bool {
        matches!(self.state, SubstreamState::PendingSend(_, _))
            || matches!(self.state, SubstreamState::PendingFlush(_))
    }

    /// Returns `true` if the substream is disabled.
    pub fn is_disabled(&self) -> bool {
        matches!(self.state, SubstreamState::Disabled)
    }
}

impl Service for DownstreamHandler {
    type InEvent = StreamHandlerIn<Framed<Stream, Codec>>;
    type OutEvent = ();

    fn poll(
        &mut self,
        svc_cx: &mut PollCtx<'_, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        loop {
            match std::mem::replace(&mut self.state, SubstreamState::Poisoned) {
                // Disabled state
                SubstreamState::Disabled => {
                    if let Some(event) = svc_cx.pop_next() {
                        if let StreamHandlerIn::Init(stream) = event {
                            tracing::debug!("Initializing outbound substream");
                            self.state = SubstreamState::Idle(stream);
                            continue;
                        } else {
                            tracing::trace!("Dropping input event: substream handler not ready");
                        }
                    }

                    self.state = SubstreamState::Disabled;
                    break;
                }
                // Idle state
                SubstreamState::Idle(stream) => {
                    if let Some(StreamHandlerIn::SendFrame(message)) = svc_cx.pop_next() {
                        self.state = SubstreamState::PendingSend(stream, message);
                        continue;
                    }

                    self.state = SubstreamState::Idle(stream);
                    break;
                }
                SubstreamState::PendingSend(mut stream, bytes) => {
                    match Sink::poll_ready(Pin::new(&mut stream), cx) {
                        Poll::Ready(Ok(())) => {
                            match Sink::start_send(Pin::new(&mut stream), bytes) {
                                Ok(()) => self.state = SubstreamState::PendingFlush(stream),
                                Err(err) => {
                                    tracing::debug!(
                                        "Failed to send message on outbound stream: {}",
                                        err
                                    );
                                    self.state = SubstreamState::Disabled;
                                    break;
                                }
                            }
                        }
                        Poll::Ready(Err(err)) => {
                            tracing::debug!("Failed to send message on outbound stream: {}", err);
                            self.state = SubstreamState::Disabled;
                            break;
                        }
                        Poll::Pending => {
                            self.state = SubstreamState::PendingSend(stream, bytes);
                            break;
                        }
                    }
                }
                SubstreamState::PendingFlush(mut stream) => {
                    match Sink::poll_flush(Pin::new(&mut stream), cx) {
                        Poll::Ready(Ok(())) => self.state = SubstreamState::Idle(stream),
                        Poll::Ready(Err(err)) => {
                            tracing::debug!("Failed to flush outbound stream: {}", err);
                            self.state = SubstreamState::Disabled;
                            break;
                        }
                        Poll::Pending => {
                            self.state = SubstreamState::PendingFlush(stream);
                            break;
                        }
                    }
                }
                SubstreamState::Poisoned => {
                    unreachable!("Error occurred during outbound stream processing")
                }
            }
        }

        Poll::Pending
    }
}
