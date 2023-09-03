use common::service::{OnEventCtx, Service};

use crate::framing::Frame;

use super::events::{DownstreamInEvent, DownstreamOutEvent};

/// The downstream framing service is responsible for encoding the messages and subscription
/// requests into frames and sending them to the destination peer.
#[derive(Default)]
pub struct DownstreamFramingService;

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
                ])
                .into();
                svc_cx.emit(DownstreamOutEvent::SendFrame { dest, frame });
            }
            DownstreamInEvent::SendSubscriptionRequest { dest, actions } => {
                // Create a new frame with the subscription actions, encode it and send it to the
                // destination peer. The resulting frame will contain only subscription actions.
                let frame = Frame::new_with_subscriptions(actions).into();
                svc_cx.emit(DownstreamOutEvent::SendFrame { dest, frame });
            }
        }
    }
}
