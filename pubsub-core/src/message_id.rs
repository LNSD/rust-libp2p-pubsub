use bytes::Bytes;
use libp2p::identity::PeerId;
use smallvec::SmallVec;

use crate::topic::TopicHash;

/// The message id is used to uniquely identify a message.
///
/// Backed by a 32 bytes `SmallVec` to avoid heap allocations for ID sizes up to 256 bits.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageId(SmallVec<[u8; 32]>);

impl MessageId {
    /// Create a new `MessageId` from an vector of bytes.
    pub fn new<T: Into<Vec<u8>>>(value: T) -> Self {
        Self(SmallVec::from(value.into()))
    }

    /// Create a new `MessageId` from a slice of bytes.
    pub fn new_from_slice(value: &[u8]) -> Self {
        Self(SmallVec::from(value))
    }

    /// Convert the `MessageId` into a `Vec<u8>`.
    fn into_vec(self) -> Vec<u8> {
        self.0.into_vec()
    }
}

impl From<Vec<u8>> for MessageId {
    /// Convert a `Vec<u8>` into a `MessageId`.
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

impl From<MessageId> for Vec<u8> {
    /// Convert a `MessageId` into a `Vec<u8>`.
    fn from(value: MessageId) -> Self {
        value.into_vec()
    }
}

impl From<Bytes> for MessageId {
    /// Convert a `Bytes` into a `MessageId`.
    fn from(value: Bytes) -> Self {
        Self::new(value)
    }
}

impl From<MessageId> for Bytes {
    /// Convert a `MessageId` into a `Bytes`.
    fn from(value: MessageId) -> Self {
        Bytes::from(value.into_vec())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", hex_fmt::HexFmt(&self.0)))
    }
}

impl std::fmt::Debug for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("MessageId({})", hex_fmt::HexFmt(&self.0)))
    }
}

/// A message is a piece of data published on a topic.
///
/// This is a immutable reference wrapper around the internal message type that provides a more
/// convenient interface to the message data while decoupling the internal message type from the
/// public API.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageRef {
    /// The message author.
    ///
    /// > NOTE: Do not confuse with the node that forwarded the message.
    pub from: Option<PeerId>,
    /// The actual message data.
    pub data: Bytes,
    /// The sequence number of the message.
    pub seqno: Option<Bytes>,
    /// The topic the message was published on.
    pub topic: TopicHash,
    /// The signature of the message.
    pub signature: Option<Bytes>,
    /// The key of the message.
    pub key: Option<Bytes>,
}

impl From<&crate::framing::Message> for MessageRef {
    fn from(msg: &crate::framing::Message) -> Self {
        MessageRef {
            from: msg.author(),
            data: msg.data(),
            seqno: msg.seqno(),
            topic: msg.topic(),
            signature: msg.signature(),
            key: msg.key(),
        }
    }
}

/// The default message id function as defined in the libp2p pusub spec.
///
/// The default message id is computed as: author + sequence number.
///
/// NOTE: If either the message author is not provided, we set it to 0.
pub fn default_message_id_fn(_src: Option<&PeerId>, msg: &MessageRef) -> MessageId {
    // If either the peer_id or source is not provided, we set to 0
    let mut source_string = if let Some(peer_id) = msg.from {
        peer_id.to_base58().into_bytes()
    } else {
        PeerId::from_bytes(&[0, 1, 0])
            .unwrap()
            .to_base58()
            .into_bytes()
    };
    source_string.extend(msg.seqno.as_ref().unwrap_or(&Bytes::new()));
    MessageId::new(source_string)
}

// NOTE: Use `trait_set` crate as `trait_alias` is not yet stable.
//       https://github.com/rust-lang/rust/issues/41517
trait_set::trait_set! {
    /// The message id function type.
    ///
    /// The message id function is used to compute the message id from a message.
    pub trait MessageIdFn = Fn(Option<&PeerId>, &MessageRef) -> MessageId;
}

#[cfg(test)]
mod tests {
    use rand::random;

    use crate::framing::Message as FrameMessage;
    use crate::topic::IdentTopic;

    use super::*;

    /// Helper function to create a random topic.
    fn new_test_topic() -> IdentTopic {
        IdentTopic::new(format!("/test-{}/0.1.0", random::<u32>()))
    }

    /// Helper function to create a random sequence number.
    fn new_test_seqno() -> Bytes {
        Bytes::from(random::<u64>().to_be_bytes().to_vec())
    }

    fn new_test_message(author: Option<PeerId>, seqno: Option<Bytes>) -> FrameMessage {
        let mut message = FrameMessage::new(new_test_topic(), b"test-data".to_vec());
        message.set_author(author);
        message.set_seqno(seqno);
        message
    }

    #[test]
    fn default_message_id_fn_should_return_same_id_for_same_message() {
        //// Given
        let source = PeerId::random();
        let message = new_test_message(Some(source), Some(new_test_seqno()));

        let id_fn: Box<dyn MessageIdFn<Output = MessageId>> = Box::new(default_message_id_fn);

        //// When
        let message_id = id_fn(None, &message.as_ref().into());
        let message_id2 = id_fn(None, &message.as_ref().into());

        //// Then
        assert_eq!(message_id, message_id2);
    }

    #[test]
    fn default_message_id_fn_should_return_same_id_for_same_message_no_source() {
        //// Given
        let message = new_test_message(None, Some(new_test_seqno()));

        let id_fn: Box<dyn MessageIdFn<Output = MessageId>> = Box::new(default_message_id_fn);

        //// When
        let message_id = id_fn(None, &message.as_ref().into());
        let message_id2 = id_fn(None, &message.as_ref().into());

        //// Then
        assert_eq!(message_id, message_id2);
    }
}
