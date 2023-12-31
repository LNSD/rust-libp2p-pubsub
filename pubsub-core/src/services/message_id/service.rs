use std::collections::HashMap;
use std::rc::Rc;

use libp2p_pubsub_common::service::{EventHandler, OnEventCtx};

use crate::message_id::{default_message_id_fn, MessageId, MessageIdFn};
use crate::topic::TopicHash;

use super::events::{MessageEvent, ServiceIn, ServiceOut, SubscriptionEvent};

/// The `MessageIdService` is responsible for generating the `MessageID` for each message. The
/// `MessageID` is used to deduplicate messages.
///
/// The `MessageID` is generated by the `MessageID` function associated with the topic of the
/// message. If at the moment the node subscribes to a topic there is no `MessageID` function
/// is provided, the [default `MessageID` function](crate::message_id::default_message_id_fn) is
/// used. If the node is not subscribed to the topic, the message id is computed using the
/// default `MessageID` function.
#[derive(Default)]
pub struct MessageIdService {
    /// A table mapping the Topic with the `MessageID` function.
    message_id_fn: HashMap<TopicHash, Rc<dyn MessageIdFn<Output = MessageId>>>,
}

impl EventHandler for MessageIdService {
    type InEvent = ServiceIn;
    type OutEvent = ServiceOut;

    fn on_event<'a>(
        &mut self,
        svc_cx: &mut impl OnEventCtx<'a, Self::OutEvent>,
        ev: Self::InEvent,
    ) {
        match ev {
            ServiceIn::SubscriptionEvent(SubscriptionEvent::Subscribed {
                message_id_fn,
                topic,
            }) => {
                // Register the topic's message id function
                let message_id_fn = message_id_fn.unwrap_or(Rc::new(default_message_id_fn));
                self.message_id_fn.insert(topic, message_id_fn);
            }
            ServiceIn::SubscriptionEvent(SubscriptionEvent::Unsubscribed(topic)) => {
                // Unregister the topic's message id function
                self.message_id_fn.remove(&topic);
            }
            ServiceIn::MessageEvent(MessageEvent::Published(message)) => {
                let message_id = match self.message_id_fn.get(&message.topic()) {
                    None => default_message_id_fn(None, &message.as_ref().into()),
                    Some(id_fn) => id_fn(None, &message.as_ref().into()),
                };

                // Emit the message event with the message id.
                svc_cx.emit(ServiceOut::MessagePublished {
                    message,
                    message_id,
                });
            }
            ServiceIn::MessageEvent(MessageEvent::Received { src, message }) => {
                let message_id = match self.message_id_fn.get(&message.topic()) {
                    None => default_message_id_fn(Some(&src), &message.as_ref().into()),
                    Some(id_fn) => id_fn(Some(&src), &message.as_ref().into()),
                };

                // Emit the message event with the message id.
                svc_cx.emit(ServiceOut::MessageReceived {
                    src,
                    message,
                    message_id,
                });
            }
        }
    }
}
