use std::collections::VecDeque;
use std::task::{Context, Poll};

use asynchronous_codec::Framed;
use bytes::Bytes;
use libp2p::Stream;

use libp2p_pubsub_common::service::{BufferedContext, PollCtx, Service, ServiceContext};

use super::codec::Codec;
use super::events_stream_handler::{StreamHandlerIn, StreamHandlerOut};
use super::send_only_stream_handler::SendOnlyStreamHandler;

#[allow(clippy::large_enum_variant)]
pub enum DownstreamIn {
    /// Send bytes to the downstream.
    Send(Bytes),
    /// A connection handler event,
    ConnHandlerEvent(DownstreamConnHandlerInEvent),
}

#[allow(clippy::large_enum_variant)]
pub enum DownstreamConnHandlerInEvent {
    /// The substream has been fully negotiated.
    FullyNegotiated(Framed<Stream, Codec>),
    /// The substream upgrade failed.
    UpradeError,
}

pub enum DownstreamOut {
    /// Acknowledge the send action.
    SendAck,
    /// A connection handler event.
    ConnHandlerEvent(DownstreamConnHandlerOutEvent),
}

pub enum DownstreamConnHandlerOutEvent {
    /// Request a new outbound substream.
    RequestNewSubstream,
}

#[derive(Debug)]
pub enum DownstreamError {
    /// The stream upgrade failed.
    UpgradeError,
    /// The maximum number of send retries has been reached.
    MaxRetriesReached,
}

pub struct Downstream {
    /// The outbound substream.
    outbound_substream: Option<BufferedContext<SendOnlyStreamHandler>>,
    /// If the outbound substream is currently being negotiated.
    outbound_substream_requested: bool,
    /// The send queue.
    send_queue: VecDeque<Bytes>,
    /// The maximum number of send retries.
    max_send_retries: usize,
    /// The number of send retries.
    send_retries: usize,
}

impl Downstream {
    pub fn new(max_send_retries: usize) -> Self {
        Self {
            max_send_retries,
            outbound_substream: None,
            outbound_substream_requested: false,
            send_retries: 0,
            send_queue: VecDeque::new(),
        }
    }

    /// Returns `true` if the downstream is currently sending bytes.
    pub fn is_sending(&self) -> bool {
        matches!(self.outbound_substream, Some(ref s) if s.is_sending())
    }
}

impl Service for Downstream {
    type InEvent = DownstreamIn;
    type OutEvent = Result<DownstreamOut, DownstreamError>;

    fn poll(
        &mut self,
        mut svc_cx: PollCtx<'_, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        // Process input events.
        if let Some(ev) = svc_cx.pop_next() {
            match ev {
                DownstreamIn::ConnHandlerEvent(DownstreamConnHandlerInEvent::FullyNegotiated(
                    stream,
                )) => {
                    self.outbound_substream_requested = false;
                    self.outbound_substream =
                        Some(BufferedContext::new(SendOnlyStreamHandler::new(stream)));
                }
                DownstreamIn::ConnHandlerEvent(DownstreamConnHandlerInEvent::UpradeError) => {
                    self.outbound_substream_requested = false;
                    self.outbound_substream = None;

                    return Poll::Ready(Err(DownstreamError::UpgradeError));
                }
                DownstreamIn::Send(bytes) => {
                    self.send_queue.push_back(bytes);
                }
            }
        }

        // If there are items in the send queue and there is no outbound substream, request a new
        // outbound substream.
        if !self.send_queue.is_empty()
            && self.outbound_substream.is_none()
            && !self.outbound_substream_requested
        {
            tracing::trace!("Requesting new outbound substream");

            self.outbound_substream_requested = true;
            return Poll::Ready(Ok(DownstreamOut::ConnHandlerEvent(
                DownstreamConnHandlerOutEvent::RequestNewSubstream,
            )));
        }

        if let Some(mut outbound_substream) = self.outbound_substream.take() {
            // If the outbound substream is idle, send the next byte sequence.
            if outbound_substream.is_idle() {
                if let Some(bytes) = self.send_queue.front() {
                    outbound_substream.do_send(StreamHandlerIn::Send(bytes.clone()));
                }
            }

            if let Poll::Ready(ev) = outbound_substream.poll(cx) {
                match ev {
                    Ok(StreamHandlerOut::SendAck) => {
                        // Reset the send retries and re-insert the outbound substream.
                        self.send_retries = 0;
                        self.outbound_substream = Some(outbound_substream);

                        // Drop the sent bytes.
                        self.send_queue.pop_front();

                        return Poll::Ready(Ok(DownstreamOut::SendAck));
                    }
                    Err(err) => {
                        tracing::debug!("send failed: {}", err);

                        self.outbound_substream = None;

                        // If the maximum number of send retries has been reached, return an error,
                        // otherwise increment the retries counter.
                        if self.send_retries >= self.max_send_retries {
                            return Poll::Ready(Err(DownstreamError::MaxRetriesReached));
                        } else {
                            self.send_retries += 1;
                        }

                        // Request a new outbound substream.
                        self.outbound_substream_requested = true;
                        return Poll::Ready(Ok(DownstreamOut::ConnHandlerEvent(
                            DownstreamConnHandlerOutEvent::RequestNewSubstream,
                        )));
                    }
                    _ => unreachable!("unexpected event: {:?}", ev),
                }
            }
        }

        Poll::Pending
    }
}
