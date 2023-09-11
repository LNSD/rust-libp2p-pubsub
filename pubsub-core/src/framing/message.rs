use bytes::Bytes;
use libp2p::identity::PeerId;

use libp2p_pubsub_proto::pubsub::MessageProto;

use crate::topic::TopicHash;

#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub(crate) proto: MessageProto,
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
        seq_no: impl Into<Vec<u8>>,
    ) -> Self {
        let mut rpc = Self::new(topic, data);
        rpc.set_sequence_number(Some(Bytes::from(seq_no.into())));
        rpc
    }

    #[must_use]
    pub fn new_with_seq_no_and_source(
        topic: impl Into<TopicHash>,
        data: impl Into<Vec<u8>>,
        seq_no: impl Into<Vec<u8>>,
        source: PeerId,
    ) -> Self {
        let mut rpc = Self::new_with_sequence_number(topic, data, seq_no);
        rpc.set_source(Some(source));
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
    pub fn sequence_number(&self) -> Option<Bytes> {
        self.proto.seqno.clone()
    }

    pub fn set_sequence_number(&mut self, seq_no: Option<impl Into<Vec<u8>>>) {
        self.proto.seqno = seq_no.map(|n| n.into().into());
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
