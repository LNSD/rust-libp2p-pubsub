use bytes::Bytes;
use libp2p::identity::PeerId;

use libp2p_pubsub_proto::pubsub::MessageProto;

use crate::topic::TopicHash;

/// A message that can be sent or received on a pubsub topic.
///
/// This type is implemented as a wrapper around the protobuf message.
#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub(crate) proto: MessageProto,
}

impl Message {
    /// Creates a new message.
    ///
    /// This method sets the only two fields that are required for a message to be valid: the topic
    /// and the data.
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

    /// Creates a new message with a sequence number.
    #[must_use]
    pub fn new_with_sequence_number(
        topic: impl Into<TopicHash>,
        data: impl Into<Vec<u8>>,
        seq_no: impl Into<Vec<u8>>,
    ) -> Self {
        let mut rpc = Self::new(topic, data);
        rpc.set_seqno(Some(Bytes::from(seq_no.into())));
        rpc
    }

    /// Creates a new message with a sequence number and an author.
    #[must_use]
    pub fn new_with_seq_no_and_from(
        topic: impl Into<TopicHash>,
        data: impl Into<Vec<u8>>,
        seq_no: impl Into<Vec<u8>>,
        from: PeerId,
    ) -> Self {
        let mut rpc = Self::new_with_sequence_number(topic, data, seq_no);
        rpc.set_author(Some(from));
        rpc
    }

    /// Converts the message into the underlying protobuf message.
    #[must_use]
    pub fn into_proto(self) -> MessageProto {
        self.proto
    }

    /// Returns a reference to the underlying protobuf message.
    #[must_use]
    pub fn as_proto(&self) -> &MessageProto {
        &self.proto
    }

    /// Returns the message author.
    ///
    /// > NOTE: Do not confuse with the node that forwarded the message.
    #[must_use]
    pub fn author(&self) -> Option<PeerId> {
        self.proto
            .from
            .as_ref()
            .map(|bytes| PeerId::from_bytes(bytes).expect("valid peer id"))
    }

    /// Sets the message author.
    pub fn set_author(&mut self, source: Option<PeerId>) {
        self.proto.from = source.map(|peer_id| peer_id.to_bytes().into());
    }

    /// Returns the message payload.
    #[must_use]
    pub fn data(&self) -> Bytes {
        self.proto
            .data
            .clone()
            .expect("message data must be present")
    }

    /// Returns the message sequence number bytes when present.
    #[must_use]
    pub fn seqno(&self) -> Option<Bytes> {
        self.proto.seqno.clone()
    }

    /// Set the message sequence number.
    pub fn set_seqno(&mut self, seq_no: Option<impl Into<Vec<u8>>>) {
        self.proto.seqno = seq_no.map(|n| Bytes::from(n.into()));
    }

    /// Returns the topic.
    #[must_use]
    pub fn topic(&self) -> TopicHash {
        TopicHash::from_raw(&self.proto.topic)
    }

    /// Returns the topic as a string slice.
    #[must_use]
    pub fn topic_str(&self) -> &str {
        self.proto.topic.as_str()
    }

    /// Returns the message signature bytes when present.
    #[must_use]
    pub fn signature(&self) -> Option<Bytes> {
        self.proto.signature.clone()
    }

    /// Sets the message signature bytes.
    pub fn set_signature(&mut self, signature: Option<impl Into<Vec<u8>>>) {
        self.proto.signature = signature.map(|bytes| bytes.into().into());
    }

    /// Returns the message key bytes when present.
    #[must_use]
    pub fn key(&self) -> Option<Bytes> {
        self.proto.key.clone()
    }

    /// Sets the message key bytes.
    pub fn set_key(&mut self, key: Option<impl Into<Vec<u8>>>) {
        self.proto.key = key.map(|bytes| bytes.into().into());
    }
}

impl AsRef<Message> for Message {
    fn as_ref(&self) -> &Self {
        self
    }
}
