use std::rc::Rc;

use bytes::Bytes;
use libp2p::identity::PeerId;
use prost::Message;

use libp2p_pubsub_common::service::{OnEventCtx, Service};
use libp2p_pubsub_proto::pubsub::{FrameProto as RawFrame, MessageProto, SubOptsProto};

use crate::framing::{Message as FrameMessage, SubscriptionAction};

use super::events::{UpstreamInEvent, UpstreamOutEvent};
use super::validation::{validate_frame_proto, validate_message_proto, validate_subopts_proto};

/// The upstream framing service is responsible for decoding, validating and processing the
/// received frames and emitting the  received messages and subscription request events.
#[derive(Default)]
pub struct UpstreamFramingService;

/// Decode a pubsub frame from a byte buffer.
fn decode_frame(frame: Bytes) -> anyhow::Result<RawFrame> {
    RawFrame::decode(frame).map_err(anyhow::Error::from)
}

/// Validate, sanitize and process a raw frame received from the `src` peer.
fn process_raw_frame(
    src: PeerId,
    frame: RawFrame,
) -> anyhow::Result<(
    impl IntoIterator<Item = FrameMessage>,
    impl IntoIterator<Item = SubscriptionAction>,
)> {
    // 1. Validate the RPC frame.
    if let Err(err) = validate_frame_proto(&frame) {
        return Err(anyhow::Error::from(err));
    }

    tracing::trace!(%src, "Frame received");

    // 2. Validate, sanitize and process the frame messages'.
    let messages_iter = process_raw_frame_messages(src, frame.publish);

    // 3. Validate, sanitize and process the frame subscription actions.
    let subscriptions_iter = process_raw_frame_subscription_requests(src, frame.subscriptions);

    // 4. Validate, sanitize and process the frame control messages.
    // TODO: Implement the frame control messages processing.

    Ok((messages_iter, subscriptions_iter))
}

/// Validates, sanitizes and processes the raw frame messages.
fn process_raw_frame_messages(
    src: PeerId,
    messages: Vec<MessageProto>,
) -> impl IntoIterator<Item = FrameMessage> {
    messages.into_iter().filter_map(move |msg| {
        if let Err(err) = validate_message_proto(&msg) {
            tracing::trace!(%src, "Received invalid message: {}", err);
            return None;
        }

        tracing::trace!(%src, "Message received");

        Some(FrameMessage::from(msg))
    })
}

/// Validates, sanitizes and processes the raw frame subscription requests.
fn process_raw_frame_subscription_requests(
    src: PeerId,
    subscriptions: Vec<SubOptsProto>,
) -> impl IntoIterator<Item = SubscriptionAction> {
    subscriptions.into_iter().filter_map(move |sub| {
        if let Err(err) = validate_subopts_proto(&sub) {
            tracing::trace!(%src, "Received invalid subscription action: {}", err);
            return None;
        }

        tracing::trace!(%src, "Subscription action received");

        Some(SubscriptionAction::from(sub))
    })
}

impl Service for UpstreamFramingService {
    type InEvent = UpstreamInEvent;
    type OutEvent = UpstreamOutEvent;

    fn on_event(&mut self, svc_cx: &mut OnEventCtx<'_, Self::OutEvent>, ev: Self::InEvent) {
        match ev {
            UpstreamInEvent::RawFrameReceived { src, frame } => {
                // Decode the received frame.
                let frame = match decode_frame(frame) {
                    Ok(frame) => frame,
                    Err(err) => {
                        tracing::trace!(%src, "Invalid frame received: {}", err);
                        return;
                    }
                };

                // Process the received frames.
                match process_raw_frame(src, frame) {
                    Ok((messages, subscriptions)) => {
                        // Emit the received messages.
                        let messages =
                            messages
                                .into_iter()
                                .map(|message| UpstreamOutEvent::MessageReceived {
                                    src,
                                    message: Rc::new(message),
                                });
                        svc_cx.emit_batch(messages);

                        // Emit the received subscription actions.
                        let subscriptions = subscriptions.into_iter().map(|action| {
                            UpstreamOutEvent::SubscriptionRequestReceived { src, action }
                        });
                        svc_cx.emit_batch(subscriptions);
                    }
                    Err(err) => {
                        tracing::trace!(%src, "Invalid frame received: {}", err);
                    }
                }
            }
        }
    }
}
