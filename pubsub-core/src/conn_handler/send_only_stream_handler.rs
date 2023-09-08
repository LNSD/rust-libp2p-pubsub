use std::pin::Pin;
use std::task::{Context, Poll};

use asynchronous_codec::Framed;
use bytes::Bytes;
use futures::Sink;
use libp2p::swarm::Stream;

use libp2p_pubsub_common::service::{PollCtx, Service};

use super::codec::{Codec, Error};
use super::events_stream_handler::{StreamHandlerError, StreamHandlerIn, StreamHandlerOut};

/// State of the outbound substream, opened either by us or by the remote.
enum SubstreamState {
    /// Waiting for the user to send a message. The idle state for an outbound substream.
    Idle(Framed<Stream, Codec>),
    /// Waiting to send a message to the remote.
    PendingSend(Framed<Stream, Codec>, Bytes),
    /// Waiting to flush the substream so that the data arrives to the remote.
    PendingFlush(Framed<Stream, Codec>),
    /// Disabled state.
    Disabled,
    /// An error occurred during processing.
    Poisoned,
}

pub struct SendOnlyStreamHandler {
    state: SubstreamState,
}

impl SendOnlyStreamHandler {
    /// Creates a new `DownstreamHandler` with the given stream.
    pub fn new(stream: Framed<Stream, Codec>) -> Self {
        Self {
            state: SubstreamState::Idle(stream),
        }
    }

    /// Returns `true` if the substream is idle.
    pub fn is_idle(&self) -> bool {
        matches!(self.state, SubstreamState::Idle(_))
    }

    /// Returns `true` if the substream is in the process of sending a message.
    pub fn is_sending(&self) -> bool {
        matches!(self.state, SubstreamState::PendingSend(_, _))
            || matches!(self.state, SubstreamState::PendingFlush(_))
    }
}

impl Service for SendOnlyStreamHandler {
    type InEvent = StreamHandlerIn;
    type OutEvent = Result<StreamHandlerOut, StreamHandlerError>;

    fn poll(
        &mut self,
        mut svc_cx: PollCtx<'_, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        loop {
            match std::mem::replace(&mut self.state, SubstreamState::Poisoned) {
                // Idle state
                SubstreamState::Idle(stream) => {
                    if let Some(StreamHandlerIn::Send(message)) = svc_cx.pop_next() {
                        tracing::trace!("Sending message on outbound stream");
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

                                    // Emit an error event.
                                    if let Error::IoError(err) = err {
                                        svc_cx.emit(Err(StreamHandlerError::SendDataFailed(err)));
                                    }

                                    self.state = SubstreamState::Disabled;
                                    break;
                                }
                            }
                        }
                        Poll::Ready(Err(err)) => {
                            tracing::debug!("Failed to send message on outbound stream: {}", err);

                            // Emit an error event.
                            if let Error::IoError(err) = err {
                                svc_cx.emit(Err(StreamHandlerError::SendDataFailed(err)));
                            }

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
                        Poll::Ready(Ok(())) => {
                            tracing::trace!("Successfully flushed outbound stream");

                            // Notify the connection handler about the sent frame.
                            svc_cx.emit(Ok(StreamHandlerOut::SendAck));

                            self.state = SubstreamState::Idle(stream);
                        }
                        Poll::Ready(Err(err)) => {
                            tracing::debug!("Failed to flush outbound stream: {}", err);

                            // Emit an error event.
                            if let Error::IoError(err) = err {
                                svc_cx.emit(Err(StreamHandlerError::FlushFailed(err)));
                            }

                            self.state = SubstreamState::Disabled;
                            break;
                        }
                        Poll::Pending => {
                            self.state = SubstreamState::PendingFlush(stream);
                            break;
                        }
                    }
                }
                SubstreamState::Disabled => {
                    // The substream is disabled. This is the terminal state.
                    self.state = SubstreamState::Disabled;
                    break;
                }
                SubstreamState::Poisoned => {
                    unreachable!("Error occurred during outbound stream processing")
                }
            }
        }

        Poll::Pending
    }
}
