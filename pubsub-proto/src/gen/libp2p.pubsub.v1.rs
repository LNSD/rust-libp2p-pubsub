// @generated
// ! Libp2p Pubsub protobuf definitions.
// !
// ! Current definitions are based on libp2p Pubsub and Gossipsub specs.
// ! See for more details:
// !  - <https://github.com/libp2p/specs/tree/master/pubsub/README.md#the-rpc>
// !  - <https://github.com/libp2p/specs/tree/master/pubsub/README.md#the-message>
// !  - <https://github.com/libp2p/specs/tree/master/pubsub/README.md#the-topic-descriptor>
// !  - <https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md#protobuf>

///
/// Communication between peers happens in the form of exchanging protobuf `Frame` messages between participating
/// peers.
///
/// The `Frame` message is a relatively simple protobuf message containing zero or more subscription action messages,
/// zero or more data messages, or zero or more control messages.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Frame {
    ///
    /// Subscription action messages.
    #[prost(message, repeated, tag="1")]
    pub subscriptions: ::prost::alloc::vec::Vec<SubOpts>,
    ///
    /// Data messages.
    #[prost(message, repeated, tag="2")]
    pub publish: ::prost::alloc::vec::Vec<Message>,
    ///
    /// Control messages.
    #[prost(message, optional, tag="3")]
    pub control: ::core::option::Option<ControlMessage>,
}
///
/// The `SubOpts` message is used to subscribe or unsubscribe from a topic.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubOpts {
    ///
    ///   The `subscribe` field indicates whether the message is a subscription or unsubscription.
    #[prost(bool, optional, tag="1")]
    pub subscribe: ::core::option::Option<bool>,
    ///
    /// The `topic_id` field specifies the topic that the message is subscribing or unsubscribing from.
    #[prost(string, optional, tag="2")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    ///
    /// The `from` field (optional) denotes the author of the message.
    ///
    /// This is the peer who initially authored the message, and NOT the peer who propagated it. Thus, as the message is
    /// routed through a swarm of pubsubbing peers, the original authorship is preserved.
    #[prost(bytes="bytes", optional, tag="1")]
    pub from: ::core::option::Option<::prost::bytes::Bytes>,
    ///
    /// The `data` field (optional) contains the payload of the message.
    ///
    /// This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a message is
    /// determined by the pubsub implementation.
    #[prost(bytes="bytes", optional, tag="2")]
    pub data: ::core::option::Option<::prost::bytes::Bytes>,
    ///
    /// The `seqno` field (optional) contains a sequence number for the message.
    ///
    /// No two messages on a pubsub topic from the same peer have the same `seqno` value, however messages from
    /// different peers may have the same sequence number. In other words, this number is not globally unique.
    ///
    /// This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a sequence number
    /// is determined by the pubsub implementation.
    #[prost(bytes="bytes", optional, tag="3")]
    pub seqno: ::core::option::Option<::prost::bytes::Bytes>,
    ///
    /// The `topic` field specifies the topic that the message should be published to.
    #[prost(string, tag="4")]
    pub topic: ::prost::alloc::string::String,
    ///
    /// The `signature` field (optional) contains a signature of the message.
    ///
    /// This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a signature is
    /// determined by the pubsub implementation.
    #[prost(bytes="bytes", optional, tag="5")]
    pub signature: ::core::option::Option<::prost::bytes::Bytes>,
    ///
    /// The `key` field (optional) contains a public key that can be used to verify the signature.
    ///
    /// This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a key is
    /// determined by the pubsub implementation.
    #[prost(bytes="bytes", optional, tag="6")]
    pub key: ::core::option::Option<::prost::bytes::Bytes>,
}
///
/// The `ControlMessage` message is used to send control messages between peers.
///
/// It contains one or more control messages.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlMessage {
    ///
    /// The `ihave` field contains a list of `ControlIHave` messages.
    #[prost(message, repeated, tag="1")]
    pub ihave: ::prost::alloc::vec::Vec<ControlIHave>,
    ///
    /// The `iwant` field contains a list of `ControlIWant` messages.
    #[prost(message, repeated, tag="2")]
    pub iwant: ::prost::alloc::vec::Vec<ControlIWant>,
    ///
    /// The `graft` field contains a list of `ControlGraft` messages.
    #[prost(message, repeated, tag="3")]
    pub graft: ::prost::alloc::vec::Vec<ControlGraft>,
    ///
    /// The `prune` field contains a list of `ControlPrune` messages.
    #[prost(message, repeated, tag="4")]
    pub prune: ::prost::alloc::vec::Vec<ControlPrune>,
}
///
/// The `ControlIHave` message is used to advertise messages that a peer has.
///
/// It provides the remote peer with a list of messages that were recently seen by the local router. The remote peer
/// may then request the full message content with a `ControlIWant` message.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlIHave {
    ///
    /// The `topic_id` field specifies the topic that the message IDs are for.
    #[prost(string, optional, tag="1")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
    ///
    /// The `message_ids` field contains a list of message IDs that the local peer has.
    #[prost(bytes="bytes", repeated, tag="2")]
    pub message_ids: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
}
///
/// The `ControlIWant` message is used to request messages from a peer.
///
/// It provides the remote peer with a list of messages that the local peer is interested in. The requested messages IDs
/// are those that were previously announced by the remote peer in an `ControlIHave` message. The remote peer may then
/// send the full message content with a `Message` message.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlIWant {
    ///
    /// The `message_ids` field contains a list of message IDs that the local peer is interested in.
    #[prost(bytes="bytes", repeated, tag="1")]
    pub message_ids: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
}
///
/// The `ControlGraft` message is grafts a new link in a topic mesh.
///
/// The `ControlGraft` message informs a peer that it has been added to the local router's mesh view for the included
/// topic id.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlGraft {
    ///
    /// The `topic_id` field specifies the topic that the message is grafting to.
    #[prost(string, optional, tag="1")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
}
///
/// The `ControlPrune` message is used to prune a link from a topic mesh.
///
/// The `ControlPrune` message informs a peer that it has been removed from the local router's mesh view for the
/// included topic id.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlPrune {
    ///
    /// The `topic_id` field specifies the topic that the message is pruning from.
    #[prost(string, optional, tag="1")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
    ///
    /// The `peers` field contains a list of `PeerInfo` messages.
    ///
    /// It is part of the gossipsub v1.1 Peer eXchange (PX) protocol extension.
    ///
    /// gossipsub v1.1 PX
    #[prost(message, repeated, tag="2")]
    pub peers: ::prost::alloc::vec::Vec<PeerInfo>,
    ///
    /// The `backoff` field specifies the time (in seconds) that the remote peer should wait before attempting to
    /// re-graft.
    #[prost(uint64, optional, tag="3")]
    pub backoff: ::core::option::Option<u64>,
}
///
/// The `PeerInfo` message is used to provide information about a peer.
///
/// It contains the peer's ID and a signed peer record.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerInfo {
    #[prost(bytes="bytes", optional, tag="1")]
    pub peer_id: ::core::option::Option<::prost::bytes::Bytes>,
    #[prost(bytes="bytes", optional, tag="2")]
    pub signed_peer_record: ::core::option::Option<::prost::bytes::Bytes>,
}
// @@protoc_insertion_point(module)
