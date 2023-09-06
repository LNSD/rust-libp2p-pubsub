use std::pin::Pin;
use std::task::{Context, Poll};

use asynchronous_codec::Framed;
use futures::{Sink, StreamExt};
use libp2p::swarm::Stream;

use common::service::{PollCtx, Service};

use super::codec::{Codec, Error};
use super::events::{StreamHandlerIn, StreamHandlerOut};

/// State of the inbound substream.
///
/// This enum acts as a state machine for the inbound substream. It is used to process inbound
/// messages and close the substream.
#[derive(Default)]
enum SubstreamState {
    /// Disabled state.
    #[default]
    Disabled,
    /// Waiting for a message from the remote. The idle state for an inbound substream.
    Idle(Framed<Stream, Codec>),
    /// The substream is being closed.
    Closing(Framed<Stream, Codec>),
    /// An error occurred during processing.
    ///
    /// This state is used to poison the substream handler and prevent further processing.
    /// Entering this state is considered a bug, and it will panic if it is polled.
    Poisoned,
}

#[derive(Default)]
pub struct UpstreamHandler {
    state: SubstreamState,
}

impl Service for UpstreamHandler {
    type InEvent = StreamHandlerIn<Framed<Stream, Codec>>;
    type OutEvent = StreamHandlerOut;

    fn poll(
        &mut self,
        mut svc_cx: PollCtx<'_, Self::InEvent, Self::OutEvent>,
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
                SubstreamState::Idle(mut stream) => {
                    match stream.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(message))) => {
                            tracing::trace!("Received frame from inbound stream");
                            self.state = SubstreamState::Idle(stream);
                            return Poll::Ready(StreamHandlerOut::FrameReceived(message));
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
                            if let Err(err) = res {
                                // Don't close the connection but just drop the inbound substream.
                                // In case the remote has more to send, they will open up a new
                                // substream.
                                tracing::debug!("Inbound substream error while closing: {}", err);
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
                SubstreamState::Poisoned => {
                    unreachable!("Error occurred during inbound stream processing")
                }
            }
        }

        Poll::Pending
    }
}
