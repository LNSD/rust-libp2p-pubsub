//! The `Message` type represents a bit of data which is published to a pubsub topic, distributed
//! over a pubsub network and received by any node subscribed to the topic.
//!
//! This module contains the public API type of a pubsub message.

use libp2p::identity::PeerId;

use crate::topic::TopicHash;

/// A pubsub message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The author of this message.
    pub from: Option<PeerId>,
    /// The data of this message.
    pub data: Vec<u8>,
    /// The sequence number of this message.
    pub sequence_number: Option<u64>,
    /// The topic this message is published to.
    pub topic: TopicHash,
    /// The signature of this message.
    pub signature: Option<Vec<u8>>,
    /// The key of this message.
    pub key: Option<Vec<u8>>,
}

impl Message {
    /// Creates a new message.
    #[must_use]
    pub fn new(topic: impl Into<TopicHash>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            from: None,
            data: data.into(),
            sequence_number: None,
            topic: topic.into(),
            signature: None,
            key: None,
        }
    }
}

// TODO: Remove after updating the floodsub crate
impl Message {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn topic(&self) -> TopicHash {
        self.topic.clone()
    }

    pub fn source(&self) -> Option<PeerId> {
        self.from
    }

    pub fn sequence_number(&self) -> Option<u64> {
        self.sequence_number
    }

    pub fn signature(&self) -> Option<Vec<u8>> {
        self.signature.clone()
    }

    pub fn key(&self) -> Option<Vec<u8>> {
        self.key.clone()
    }

    pub fn topic_str(&self) -> &str {
        self.topic.as_str()
    }
}
