use std::rc::Rc;

use assert_matches::assert_matches;
use libp2p::identity::PeerId;
use rand::random;

use common_test as testlib;
use common_test::service::noop_context;

use crate::framing::Message as FrameMessage;
use crate::topic::TopicHash;

use super::events::{DownstreamInEvent, DownstreamOutEvent};
use super::service_downstream::DownstreamFramingService;

/// Convenience function to create a new `PeerId` for testing.
fn new_test_peer_id() -> PeerId {
    PeerId::random()
}

/// Create a new random test topic.
fn new_test_topic() -> TopicHash {
    TopicHash::from_raw(format!("/pubsub/2/it-pubsub-test-{}", random::<u32>()))
}

/// Create a test `Message` with given topic and random payload.
fn new_test_message(topic: TopicHash) -> FrameMessage {
    let payload = format!("test-payload-{}", random::<u32>());
    FrameMessage::new(topic, payload.into_bytes())
}

mod upstream {
    // TODO: Implement the upstream framing service tests.
}

mod downstream {
    use crate::framing::SubscriptionAction;

    use super::*;

    /// Convenience function to create a new `DownstreamInEvent::ForwardMessage` event sequence.
    fn new_forward_message_seq(
        dest: PeerId,
        message: FrameMessage,
    ) -> impl IntoIterator<Item = DownstreamInEvent> {
        [DownstreamInEvent::ForwardMessage {
            dest,
            message: Rc::new(message),
        }]
    }

    /// Convenience function to create a new `DownstreamInEvent::SendSubscriptionRequest` event
    /// sequence.
    fn new_send_subscription_request_seq(
        dest: PeerId,
        topics: impl IntoIterator<Item = TopicHash>,
    ) -> impl IntoIterator<Item = DownstreamInEvent> {
        [DownstreamInEvent::SendSubscriptionRequest {
            dest,
            action: topics
                .into_iter()
                .map(SubscriptionAction::Subscribe)
                .collect(),
        }]
    }

    #[test]
    fn encode_a_forwarded_message() {
        //// Given
        let remote_peer = new_test_peer_id();
        let topic = new_test_topic();
        let message = new_test_message(topic.clone());

        let mut service = testlib::service::default_test_service::<DownstreamFramingService>();

        //// When
        let input_events = new_forward_message_seq(remote_peer, message.clone());
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 1, "Only 1 event should be emitted");
        assert_matches!(&output_events[0], DownstreamOutEvent::SendFrame { dest, frame } => {
            // Assert the destination peer is the expected one.
            assert_eq!(dest, &remote_peer);
            // Assert the frame content
            assert!(frame.subscriptions.is_empty(), "No subscriptions should be encoded");
            assert!(frame.control.is_none(), "No control messages should be encoded");

            assert_eq!(frame.publish.len(), 1, "Only 1 message should be encoded");
            assert_eq!(
                &frame.publish[0].topic, topic.as_str(),
                "The encoded message topic should be the expected one"
            );
        });
    }

    #[test]
    fn encode_multiple_subscription_requests() {
        //// Given
        let remote_peer = new_test_peer_id();
        let topic_a = new_test_topic();
        let topic_b = new_test_topic();
        let topic_c = new_test_topic();

        let mut service = testlib::service::default_test_service::<DownstreamFramingService>();

        //// When
        let input_events = new_send_subscription_request_seq(
            remote_peer,
            [topic_a.clone(), topic_b.clone(), topic_c.clone()],
        );
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 1, "Only 1 event should be emitted");
        assert_matches!(&output_events[0], DownstreamOutEvent::SendFrame { dest, frame } => {
            // Assert the destination peer is the expected one.
            assert_eq!(dest, &remote_peer);
            // Assert the frame content
            assert!(frame.publish.is_empty(), "No messages should be encoded");
            assert!(frame.control.is_none(), "No control messages should be encoded");

            assert_eq!(
                frame.subscriptions.len(),
                3,
                "Only 3 subscription actions should be encoded"
            );
            assert_matches!(frame.subscriptions[0].subscribe, Some(subscribe) => {
                assert!(subscribe, "The encoded subscription action should be 'subscribe'");
            });
            assert_matches!(&frame.subscriptions[0].topic_id, Some(topic) => {
                assert_eq!(
                    topic, topic_a.as_str(),
                    "The encoded subscription action topic should be the expected one"
                );
            });
        });
    }
}
