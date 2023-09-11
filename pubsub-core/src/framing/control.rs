use libp2p::identity::PeerId;

use crate::message_id::MessageId;
use crate::topic::TopicHash;

/// A Control message exchanged between peers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlMessage {
    /// The `IHAVE` control message.
    IHave(IHaveControlMessage),
    /// The `IWant` control message.
    IWant(IWantControlMessage),
    /// The `Graft` control message.
    Graft(GraftControlMessage),
    /// The `Prune` control message.
    Prune(PruneControlMessage),
}

/// The `IHAVE` control message.
///
/// A node broadcasts known messages per topic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IHaveControlMessage {
    /// The topic of the messages.
    pub topic_hash: TopicHash,
    /// A list of known message ids.
    pub message_ids: Vec<MessageId>,
}

/// The `IWant` control message.
///
/// The node requests specific message ids from the local cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IWantControlMessage {
    /// A list of known message ids.
    pub message_ids: Vec<MessageId>,
}

/// The `Graft` control message.
///
/// Indicates that the node has been added to the senders mesh.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraftControlMessage {
    /// The mesh topic the peer should be added to.
    pub topic_hash: TopicHash,
}

/// The `Prune` control message.
///
/// The node has been removed from the senders mesh.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PruneControlMessage {
    /// The mesh topic the peer should be removed from.
    pub topic_hash: TopicHash,
    /// A list of peers to be proposed to the removed peer as peer exchange
    pub peers: Vec<PeerId>,
    /// The backoff time in seconds before we allow to reconnect
    pub backoff: Option<u64>,
}
