use std::rc::Rc;

use bytes::Bytes;
use libp2p::identity::PeerId;
use prost::Message as _;

use libp2p_pubsub_common::service::{OnEventCtx, Service};
use libp2p_pubsub_proto::pubsub::{
    ControlMessageProto, FrameProto as RawFrame, MessageProto, SubOptsProto,
};

use crate::framing::{ControlMessage, Message as FrameMessage, SubscriptionAction};

use super::events::{UpstreamInEvent, UpstreamOutEvent};
use super::validation::validate_frame_proto;

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
    impl IntoIterator<Item = ControlMessage>,
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
    let control_iter = process_raw_frame_control_messages(src, frame.control);

    Ok((messages_iter, subscriptions_iter, control_iter))
}

/// Validates, sanitizes and processes the raw frame messages.
fn process_raw_frame_messages(
    src: PeerId,
    messages: Vec<MessageProto>,
) -> impl IntoIterator<Item = FrameMessage> {
    messages
        .into_iter()
        .filter_map(move |msg| match msg.try_into() {
            Ok(msg) => {
                tracing::trace!(%src, "Message received");
                Some(msg)
            }
            Err(err) => {
                tracing::trace!(%src, "Received invalid message: {}", err);
                None
            }
        })
}

/// Validates, sanitizes and processes the raw frame subscription requests.
fn process_raw_frame_subscription_requests(
    src: PeerId,
    subscriptions: Vec<SubOptsProto>,
) -> impl IntoIterator<Item = SubscriptionAction> {
    subscriptions
        .into_iter()
        .filter_map(move |sub| match sub.try_into() {
            Ok(sub) => {
                tracing::trace!(%src, "Subscription request received");
                Some(sub)
            }
            Err(err) => {
                tracing::trace!(%src, "Received invalid subscription action: {}", err);
                None
            }
        })
}

/// Validates, sanitizes and processes the raw frame control messages.
fn process_raw_frame_control_messages(
    src: PeerId,
    control: Option<ControlMessageProto>,
) -> impl IntoIterator<Item = ControlMessage> {
    control.into_iter().flat_map(move |ctrl_msg| {
        let graft = ctrl_msg
            .graft
            .into_iter()
            .filter_map(move |ctrl| match ctrl.try_into() {
                Ok(ctrl) => Some(ControlMessage::Graft(ctrl)),
                Err(err) => {
                    tracing::trace!(%src, "Received invalid graft control message: {}", err);
                    None
                }
            });
        let prune = ctrl_msg
            .prune
            .into_iter()
            .filter_map(move |ctrl| match ctrl.try_into() {
                Ok(ctrl) => Some(ControlMessage::Prune(ctrl)),
                Err(err) => {
                    tracing::trace!(%src, "Received invalid prune control message: {}", err);
                    None
                }
            });

        let ihave = ctrl_msg
            .ihave
            .into_iter()
            .filter_map(move |ctrl| match ctrl.try_into() {
                Ok(ctrl) => Some(ControlMessage::IHave(ctrl)),
                Err(err) => {
                    tracing::trace!(%src, "Received invalid iwant control message: {}", err);
                    None
                }
            });
        let iwant = ctrl_msg
            .iwant
            .into_iter()
            .filter_map(move |ctrl| match ctrl.try_into() {
                Ok(ctrl) => Some(ControlMessage::IWant(ctrl)),
                Err(err) => {
                    tracing::trace!(%src, "Received invalid ihave control message: {}", err);
                    None
                }
            });

        itertools::chain!(graft, prune, ihave, iwant)
    })
}

impl Service for UpstreamFramingService {
    type InEvent = UpstreamInEvent;
    type OutEvent = UpstreamOutEvent;

    fn on_event<'a>(
        &mut self,
        svc_cx: &mut impl OnEventCtx<'a, Self::OutEvent>,
        ev: Self::InEvent,
    ) {
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
                    Ok((messages, subscriptions, control)) => {
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

                        // Emit the received control messages.
                        let control = control.into_iter().map(|message| {
                            UpstreamOutEvent::ControlMessageReceived { src, message }
                        });
                        svc_cx.emit_batch(control);
                    }
                    Err(err) => {
                        tracing::trace!(%src, "Invalid frame received: {}", err);
                    }
                }
            }
        }
    }
}
