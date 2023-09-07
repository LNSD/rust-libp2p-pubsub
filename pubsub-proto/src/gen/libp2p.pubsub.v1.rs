// @generated
// ! Libp2p Pubsub protobuf definitions.
// !
// ! Current definitions are based on libp2p Pubsub and Gossipsub specs.
// ! See for more details:
// !  - <https://github.com/libp2p/specs/tree/master/pubsub/README.md#the-rpc>
// !  - <https://github.com/libp2p/specs/tree/master/pubsub/README.md#the-message>
// !  - <https://github.com/libp2p/specs/tree/master/pubsub/README.md#the-topic-descriptor>
// !  - <https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md#protobuf>

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Rpc {
    #[prost(message, repeated, tag="1")]
    pub subscriptions: ::prost::alloc::vec::Vec<rpc::SubOpts>,
    #[prost(message, repeated, tag="2")]
    pub publish: ::prost::alloc::vec::Vec<Message>,
    #[prost(message, optional, tag="3")]
    pub control: ::core::option::Option<ControlMessage>,
}
/// Nested message and enum types in `RPC`.
pub mod rpc {
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SubOpts {
        /// subscribe or unsubscribe
        #[prost(bool, optional, tag="1")]
        pub subscribe: ::core::option::Option<bool>,
        #[prost(string, optional, tag="2")]
        pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(bytes="bytes", optional, tag="1")]
    pub from: ::core::option::Option<::prost::bytes::Bytes>,
    #[prost(bytes="bytes", optional, tag="2")]
    pub data: ::core::option::Option<::prost::bytes::Bytes>,
    #[prost(bytes="bytes", optional, tag="3")]
    pub seqno: ::core::option::Option<::prost::bytes::Bytes>,
    #[prost(string, tag="4")]
    pub topic: ::prost::alloc::string::String,
    #[prost(bytes="bytes", optional, tag="5")]
    pub signature: ::core::option::Option<::prost::bytes::Bytes>,
    #[prost(bytes="bytes", optional, tag="6")]
    pub key: ::core::option::Option<::prost::bytes::Bytes>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlMessage {
    #[prost(message, repeated, tag="1")]
    pub ihave: ::prost::alloc::vec::Vec<ControlIHave>,
    #[prost(message, repeated, tag="2")]
    pub iwant: ::prost::alloc::vec::Vec<ControlIWant>,
    #[prost(message, repeated, tag="3")]
    pub graft: ::prost::alloc::vec::Vec<ControlGraft>,
    #[prost(message, repeated, tag="4")]
    pub prune: ::prost::alloc::vec::Vec<ControlPrune>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlIHave {
    #[prost(string, optional, tag="1")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(bytes="bytes", repeated, tag="2")]
    pub message_ids: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlIWant {
    #[prost(bytes="bytes", repeated, tag="1")]
    pub message_ids: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlGraft {
    #[prost(string, optional, tag="1")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControlPrune {
    #[prost(string, optional, tag="1")]
    pub topic_id: ::core::option::Option<::prost::alloc::string::String>,
    /// gossipsub v1.1 PX
    #[prost(message, repeated, tag="2")]
    pub peers: ::prost::alloc::vec::Vec<PeerInfo>,
    /// gossipsub v1.1 backoff time (in seconds)
    #[prost(uint64, optional, tag="3")]
    pub backoff: ::core::option::Option<u64>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PeerInfo {
    #[prost(bytes="bytes", optional, tag="1")]
    pub peer_id: ::core::option::Option<::prost::bytes::Bytes>,
    #[prost(bytes="bytes", optional, tag="2")]
    pub signed_peer_record: ::core::option::Option<::prost::bytes::Bytes>,
}
// @@protoc_insertion_point(module)
