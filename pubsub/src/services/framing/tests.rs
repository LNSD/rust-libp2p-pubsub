use std::rc::Rc;

use assert_matches::assert_matches;
use bytes::{Bytes, BytesMut};
use libp2p::identity::PeerId;
use prost::Message;
use rand::random;

use common_test as testlib;
use common_test::service::noop_context;

use crate::framing::{Frame, FrameProto, Message as FrameMessage, SubscriptionAction};
use crate::topic::TopicHash;

use super::events::{DownstreamInEvent, DownstreamOutEvent, UpstreamInEvent, UpstreamOutEvent};
use super::service_downstream::DownstreamFramingService;
use super::service_upstream::UpstreamFramingService;

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

/// Convenience function to encode a frame into a byte buffer.
fn encode_frame(frame: impl Into<FrameProto>) -> Bytes {
    let frame = frame.into();

    let mut bytes = BytesMut::with_capacity(frame.encoded_len());
    frame.encode(&mut bytes).unwrap();
    bytes.freeze()
}

/// Convenience function to decode a pubsub frame from a byte buffer.
fn decode_frame(frame: &Bytes) -> FrameProto {
    FrameProto::decode(frame.as_ref()).expect("Failed to decode frame")
}

mod upstream {
    use super::*;

    /// Convenience function to create a new `UpstreamInEvent::RawFrameReceived` event sequence.
    fn new_raw_frame_received_seq(
        src: PeerId,
        frame: impl Into<FrameProto>,
    ) -> impl IntoIterator<Item = UpstreamInEvent> {
        [UpstreamInEvent::RawFrameReceived {
            src,
            frame: encode_frame(frame),
        }]
    }

    #[test]
    fn process_invalid_empty_frame() {
        //// Given
        let remote_peer = new_test_peer_id();
        let empty_frame = Frame::empty();

        let mut service = testlib::service::default_test_service::<UpstreamFramingService>();

        //// When
        let input_events = new_raw_frame_received_seq(remote_peer, empty_frame);
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 0, "No events should be emitted");
    }

    #[test]
    fn process_frame_with_invalid_message_empty_topic() {
        //// Given
        let remote_peer = new_test_peer_id();

        let empty_topic = TopicHash::from_raw("");
        let invalid_message = new_test_message(empty_topic);
        let frame = Frame::new_with_messages([invalid_message]);

        let mut service = testlib::service::default_test_service::<UpstreamFramingService>();

        //// When
        let input_events = new_raw_frame_received_seq(remote_peer, frame);
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 0, "No events should be emitted");
    }

    #[test]
    fn process_frame_with_invalid_subscription_request_empty_topic() {
        //// Given
        let remote_peer = new_test_peer_id();

        let empty_topic = TopicHash::from_raw("");
        let invalid_subscription_request = SubscriptionAction::Subscribe(empty_topic);
        let frame = Frame::new_with_subscriptions([invalid_subscription_request]);

        let mut service = testlib::service::default_test_service::<UpstreamFramingService>();

        //// When
        let input_events = new_raw_frame_received_seq(remote_peer, frame);
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 0, "No events should be emitted");
    }

    #[test]
    fn process_frame_with_multiple_messages() {
        //// Given
        let remote_peer = new_test_peer_id();

        let topic_a = new_test_topic();
        let topic_b = new_test_topic();

        let message_a = new_test_message(topic_a.clone());
        let message_b = new_test_message(topic_b.clone());

        let frame = Frame::new_with_messages([message_a.clone(), message_b.clone()]);

        let mut service = testlib::service::default_test_service::<UpstreamFramingService>();

        //// When
        let input_events = new_raw_frame_received_seq(remote_peer, frame);
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 2, "Only 2 events should be emitted");
        assert_matches!(&output_events[0], UpstreamOutEvent::MessageReceived { src, message } => {
            assert_eq!(src, &remote_peer);
            assert_eq!(message.as_ref(), &message_a);
        });
        assert_matches!(&output_events[1], UpstreamOutEvent::MessageReceived { src, message } => {
            assert_eq!(src, &remote_peer);
            assert_eq!(message.as_ref(), &message_b);
        });
    }

    #[test]
    fn process_frame_with_subscription_requests() {
        //// Given
        let remote_peer = new_test_peer_id();

        let topic_a = new_test_topic();
        let topic_b = new_test_topic();

        let subscription_request_a = SubscriptionAction::Subscribe(topic_a.clone());
        let subscription_request_b = SubscriptionAction::Unsubscribe(topic_b.clone());

        let frame = Frame::new_with_subscriptions([
            subscription_request_a.clone(),
            subscription_request_b.clone(),
        ]);

        let mut service = testlib::service::default_test_service::<UpstreamFramingService>();

        //// When
        let input_events = new_raw_frame_received_seq(remote_peer, frame);
        testlib::service::inject_events(&mut service, input_events);

        let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

        //// Then
        assert_eq!(output_events.len(), 2, "Only 2 events should be emitted");
        assert_matches!(&output_events[0], UpstreamOutEvent::SubscriptionRequestReceived { src, action } => {
            assert_eq!(src, &remote_peer);
            assert_eq!(action, &subscription_request_a);
        });
        assert_matches!(&output_events[1], UpstreamOutEvent::SubscriptionRequestReceived { src, action } => {
            assert_eq!(src, &remote_peer);
            assert_eq!(action, &subscription_request_b);
        });
    }
}

mod downstream {
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
            actions: topics
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
            assert!(!frame.is_empty(), "The encoded frame buffers should not be empty");

            let frame = decode_frame(frame);
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
            assert!(
                !frame.is_empty(),
                "The encoded frame buffers should not be empty"
            );

            let frame = decode_frame(frame);
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
