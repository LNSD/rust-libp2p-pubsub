use bytes::Bytes;
use libp2p::identity::PeerId;

use crate::framing::Message as FrameMessage;

/// Macro for declaring message id types
macro_rules! declare_message_id_type {
    ($name: ident) => {
        #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Vec<u8>);

        impl $name {
            pub fn new<T: Into<Vec<u8>>>(value: T) -> Self {
                Self(value.into())
            }

            pub fn new_from_slice(value: &[u8]) -> Self {
                Self(value.to_vec())
            }

            fn into_vec(self) -> Vec<u8> {
                self.0
            }
        }

        impl From<Vec<u8>> for $name {
            fn from(value: Vec<u8>) -> Self {
                Self(value)
            }
        }

        impl From<Bytes> for $name {
            fn from(value: Bytes) -> Self {
                Self(value.to_vec())
            }
        }

        impl From<$name> for Bytes {
            fn from(value: $name) -> Self {
                Bytes::from(value.into_vec())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", hex_fmt::HexFmt(&self.0))
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), hex_fmt::HexFmt(&self.0))
            }
        }
    };
}

// A type for pubsub message IDs.
declare_message_id_type!(MessageId);

pub type MessageIdFn = dyn Fn(&FrameMessage) -> MessageId + Send + Sync + 'static;

/// The default message id function as defined in the libp2p pusub spec.
///
/// The default message id is computed as: source + sequence number.
///
/// NOTE: If either the `peer_id` or `source` is not provided, we set it to 0.
pub fn default_message_id_fn(msg: &FrameMessage) -> MessageId {
    // If either the peer_id or source is not provided, we set to 0
    let mut source_string = if let Some(peer_id) = msg.source().as_ref() {
        peer_id.to_base58().into_bytes()
    } else {
        PeerId::from_bytes(&[0, 1, 0])
            .unwrap()
            .to_base58()
            .into_bytes()
    };
    source_string.extend(msg.sequence_number().unwrap_or_default());
    MessageId::new(source_string)
}

#[cfg(test)]
mod tests {
    use rand::random;

    use crate::topic::IdentTopic;

    use super::*;

    /// Helper function to create a random topic.
    fn new_test_topic() -> IdentTopic {
        IdentTopic::new(format!("/test-{}/0.1.0", random::<u32>()))
    }

    /// Helper function to create a random sequence number.
    fn new_test_seqno() -> Bytes {
        Bytes::from(random::<u32>().to_be_bytes().to_vec())
    }

    fn new_test_message(source: Option<PeerId>, seqno: Option<Bytes>) -> FrameMessage {
        let mut message = FrameMessage::new(new_test_topic(), b"test-data".to_vec());
        message.set_source(source);
        message.set_sequence_number(seqno);
        message
    }

    #[test]
    fn default_message_id_fn_should_return_same_id_for_same_message() {
        //// Given
        let source = PeerId::random();
        let message = new_test_message(Some(source), Some(new_test_seqno()));

        let id_fn: Box<MessageIdFn> = Box::new(default_message_id_fn);

        //// When
        let message_id = id_fn(&message);
        let message_id2 = id_fn(&message);

        //// Then
        assert_eq!(message_id, message_id2);
    }

    #[test]
    fn default_message_id_fn_should_return_same_id_for_same_message_no_source() {
        //// Given
        let message = new_test_message(None, Some(new_test_seqno()));

        let id_fn: Box<MessageIdFn> = Box::new(default_message_id_fn);

        //// When
        let message_id = id_fn(&message);
        let message_id2 = id_fn(&message);

        //// Then
        assert_eq!(message_id, message_id2);
    }
}
