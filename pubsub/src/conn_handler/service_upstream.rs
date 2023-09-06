use std::pin::Pin;
use std::task::{Context, Poll};

use asynchronous_codec::Framed;
use futures::{Sink, StreamExt};
use libp2p::swarm::Stream;

use common::service::{PollCtx, Service};

use super::codec::{Codec, Error};
use super::events::{StreamHandlerError, StreamHandlerIn, StreamHandlerOut};

/// State of the inbound substream.
///
/// This enum acts as a state machine for the inbound substream. It is used to process inbound
/// messages and close the substream.
enum SubstreamState {
    /// Waiting for a message from the remote. The idle state for an inbound substream.
    Idle(Framed<Stream, Codec>),
    /// The substream is being closed.
    Closing(Framed<Stream, Codec>),
    /// The substream is disabled.
    Disabled,
    /// An error occurred during processing.
    ///
    /// This state is used to poison the substream handler and prevent further processing.
    /// Entering this state is considered a bug, and it will panic if it is polled.
    Poisoned,
}

pub struct UpstreamHandler {
    state: SubstreamState,
}

impl UpstreamHandler {
    /// Creates a new `UpstreamHandler` with the given stream.
    pub fn new(stream: Framed<Stream, Codec>) -> Self {
        Self {
            state: SubstreamState::Idle(stream),
        }
    }
}

impl Service for UpstreamHandler {
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
                SubstreamState::Idle(mut stream) => {
                    match stream.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(message))) => {
                            tracing::trace!("Received frame from inbound stream");
                            self.state = SubstreamState::Idle(stream);
                            return Poll::Ready(Ok(StreamHandlerOut::FrameReceived(message)));
                        }
                        Poll::Ready(Some(Err(err))) => {
                            match err {
                                e @ Error::MaxMessageLenExceeded => {
                                    tracing::trace!("Ignoring received message: {}", e);
                                    self.state = SubstreamState::Idle(stream);
                                    continue;
                                }
                                e @ Error::LengthPrefixError(_) => {
                                    tracing::trace!("Ignoring received message: {}", e);
                                    self.state = SubstreamState::Idle(stream);
                                    continue;
                                }
                                Error::IoError(e) => {
                                    tracing::debug!("Failed to read from inbound stream: {}", e);

                                    // Emit an error event.
                                    svc_cx.emit(Err(StreamHandlerError::ReadDataFailed(e)));

                                    // Close this side of the stream. If the
                                    // peer is still around, they will re-establish their
                                    // outbound stream i.e. our inbound stream.
                                    self.state = SubstreamState::Closing(stream);
                                }
                            }
                        }
                        // peer closed the stream
                        Poll::Ready(None) => {
                            tracing::debug!("Inbound stream closed by remote");

                            // Emit an error event.
                            svc_cx.emit(Err(StreamHandlerError::ClosedByRemote));

                            self.state = SubstreamState::Closing(stream);
                        }
                        Poll::Pending => {
                            self.state = SubstreamState::Idle(stream);
                            break;
                        }
                    }
                }
                SubstreamState::Closing(mut stream) => {
                    match Sink::poll_close(Pin::new(&mut stream), cx) {
                        Poll::Ready(res) => {
                            if let Err(Error::IoError(err)) = res {
                                // Don't close the connection but just drop the inbound substream.
                                // In case the remote has more to send, they will open up a new
                                // substream.
                                tracing::debug!("Inbound substream error while closing: {}", err);

                                // Emit an error event.
                                svc_cx.emit(Err(StreamHandlerError::CloseFailed(err)));
                            }

                            self.state = SubstreamState::Disabled;
                            break;
                        }
                        Poll::Pending => {
                            self.state = SubstreamState::Closing(stream);
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
                    unreachable!("Error occurred during inbound stream processing")
                }
            }
        }

        Poll::Pending
    }
}
