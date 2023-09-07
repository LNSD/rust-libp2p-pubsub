//! The `Message` type represents a bit of data which is published to a pubsub topic, distributed
//! over a pubsub network and received by any node subscribed to the topic.
//!
//! This module contains the public API type of a pubsub message.

use bytes::Bytes;
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
    pub sequence_number: Option<Bytes>,
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

    /// Creates a new message with a sequence number.
    #[must_use]
    pub fn new_with_sequence_number(
        topic: impl Into<TopicHash>,
        data: impl Into<Vec<u8>>,
        seq_no: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            from: None,
            data: data.into(),
            sequence_number: Some(Bytes::from(seq_no.into())),
            topic: topic.into(),
            signature: None,
            key: None,
        }
    }
}
