use bytes::Bytes;
use libp2p::identity::PeerId;

use crate::framing::{validate_message_proto, MessageProto};
use crate::topic::TopicHash;

#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    proto: MessageProto,
}

impl Message {
    #[must_use]
    pub fn new(topic: impl Into<TopicHash>, data: impl Into<Vec<u8>>) -> Self {
        let topic = topic.into();
        let data = data.into();

        let proto = MessageProto {
            from: None,
            data: Some(data.into()),
            seqno: None,
            topic: topic.into_string(),
            signature: None,
            key: None,
        };

        Self { proto }
    }

    #[must_use]
    pub fn new_with_sequence_number(
        topic: impl Into<TopicHash>,
        data: impl Into<Vec<u8>>,
        seq_no: Option<u64>,
    ) -> Self {
        let mut rpc = Self::new(topic, data);
        rpc.set_sequence_number(seq_no);
        rpc
    }

    #[must_use]
    pub fn into_proto(self) -> MessageProto {
        self.proto
    }

    #[must_use]
    pub fn as_proto(&self) -> &MessageProto {
        &self.proto
    }

    #[must_use]
    pub fn source(&self) -> Option<PeerId> {
        self.proto
            .from
            .as_ref()
            .map(|bytes| PeerId::from_bytes(bytes).expect("valid peer id"))
    }

    pub fn set_source(&mut self, source: Option<PeerId>) {
        self.proto.from = source.map(|peer_id| peer_id.to_bytes().into());
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.proto.data.as_ref().unwrap()
    }

    #[must_use]
    pub fn sequence_number(&self) -> Option<u64> {
        self.proto.seqno.as_ref().map(|bytes| {
            // From pubsub spec: https://github.com/libp2p/specs/tree/master/pubsub#the-message
            // seqno field must be a 64-bit big-endian serialized unsigned integer
            let be_bytes = bytes[..].try_into().unwrap();
            u64::from_be_bytes(be_bytes)
        })
    }

    pub fn set_sequence_number(&mut self, seq_no: Option<u64>) {
        self.proto.seqno = seq_no.map(|no| no.to_be_bytes().to_vec().into());
    }

    #[must_use]
    pub fn topic_str(&self) -> &str {
        self.proto.topic.as_str()
    }

    #[must_use]
    pub fn topic(&self) -> TopicHash {
        TopicHash::from_raw(self.topic_str())
    }

    #[must_use]
    pub fn signature(&self) -> Option<&[u8]> {
        self.proto.signature.as_ref().map(|bytes| bytes.as_ref())
    }

    pub fn set_signature(&mut self, signature: Option<impl Into<Vec<u8>>>) {
        self.proto.signature = signature.map(|bytes| bytes.into().into());
    }

    #[must_use]
    pub fn key(&self) -> Option<&[u8]> {
        self.proto.key.as_ref().map(|bytes| bytes.as_ref())
    }

    pub fn set_key(&mut self, key: Option<impl Into<Vec<u8>>>) {
        self.proto.key = key.map(|bytes| bytes.into().into());
    }
}

impl From<MessageProto> for Message {
    /// Convert from a [`MessageProto`] into a [`Message`]. Additionally. sanitize the protobuf
    /// message by removing optional fields when empty.
    #[must_use]
    fn from(mut proto: MessageProto) -> Self {
        // A non-present data field should be interpreted as an empty payload.
        if proto.data.is_none() {
            proto.data = Some(Bytes::new());
        }

        // An empty from field should be interpreted as not present.
        if let Some(from) = proto.from.as_ref() {
            if from.is_empty() {
                proto.from = None;
            }
        }

        // An empty seqno field should be interpreted as not present.
        if let Some(seq_no) = proto.seqno.as_ref() {
            if seq_no.is_empty() {
                proto.seqno = None;
            }
        }

        // An empty signature field should be interpreted as not present.
        if let Some(signature) = proto.signature.as_ref() {
            if signature.is_empty() {
                proto.signature = None;
            }
        }

        // An empty key field should be interpreted as not present.
        if let Some(key) = proto.key.as_ref() {
            if key.is_empty() {
                proto.key = None;
            }
        }

        // Assert proto validity after sanitizing (development builds only)
        debug_assert!(
            validate_message_proto(&proto).is_ok(),
            "invalid message proto: {proto:?}",
        );

        Self { proto }
    }
}

impl From<Message> for MessageProto {
    fn from(message: Message) -> Self {
        message.into_proto()
    }
}
