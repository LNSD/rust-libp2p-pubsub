use bytes::{Bytes, BytesMut};
use prost::Message;

use libp2p_pubsub_common::service::{OnEventCtx, Service};
use libp2p_pubsub_proto::pubsub::FrameProto;

use crate::framing::Frame;

use super::events::{DownstreamInEvent, DownstreamOutEvent};

/// The downstream framing service is responsible for encoding the messages and subscription
/// requests into frames and sending them to the destination peer.
#[derive(Default)]
pub struct DownstreamFramingService;

/// Encode a frame into a byte buffer.
///
/// This function uses the `prost` crate to encode the frame into a byte buffer.
fn encode_frame(frame: impl Into<FrameProto>) -> Bytes {
    let frame = frame.into();

    let mut bytes = BytesMut::with_capacity(frame.encoded_len());
    frame.encode(&mut bytes).unwrap();
    bytes.freeze()
}

impl Service for DownstreamFramingService {
    type InEvent = DownstreamInEvent;
    type OutEvent = DownstreamOutEvent;

    fn on_event(&mut self, svc_cx: &mut OnEventCtx<'_, Self::OutEvent>, ev: Self::InEvent) {
        match ev {
            DownstreamInEvent::ForwardMessage { dest, message } => {
                // Create a new frame with the message, encode it and send it to the destination
                // peer. The resulting frame will contain only one message.
                let frame = Frame::new_with_messages([
                    // Clone the message as it is wrapped in an `Rc`.
                    (*message).clone(),
                ]);

                // Encode the frame into a byte buffer and send it to the destination peer.
                let frame = encode_frame(frame);
                svc_cx.emit(DownstreamOutEvent::SendFrame { dest, frame });
            }
            DownstreamInEvent::SendSubscriptionRequest { dest, actions } => {
                // Create a new frame with the subscription actions, encode it and send it to the
                // destination peer. The resulting frame will contain only subscription actions.
                let frame = Frame::new_with_subscriptions(actions);

                // Encode the frame into a byte buffer and send it to the destination peer.
                let frame = encode_frame(frame);
                svc_cx.emit(DownstreamOutEvent::SendFrame { dest, frame });
            }
        }
    }
}
