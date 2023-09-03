use std::rc::Rc;

use libp2p::identity::PeerId;

use common::service::{OnEventCtx, Service};

use crate::framing::{
    validate_frame_proto, validate_message_proto, validate_subopts_proto, FrameProto as RawFrame,
    Message as FrameMessage, MessageProto, SubOptsProto, SubscriptionAction,
};
use crate::services::framing::{UpstreamInEvent, UpstreamOutEvent};

#[derive(Default)]
pub struct UpstreamFramingService;

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

        Some(SubscriptionAction::from(sub))
    })
}

impl Service for UpstreamFramingService {
    type InEvent = UpstreamInEvent;
    type OutEvent = UpstreamOutEvent;

    fn on_event(&mut self, svc_cx: &mut OnEventCtx<'_, Self::OutEvent>, ev: Self::InEvent) {
        match ev {
            UpstreamInEvent::RawFrameReceived { src, frame } => {
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
                        tracing::trace!(%src, "Received an invalid frame: {}", err);
                    }
                }
            }
        }
    }
}
