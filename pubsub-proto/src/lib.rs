mod pb {
    include!(concat!(env!("OUT_DIR"), "/proto/mod.rs"));
}

pub mod topic {
    pub use super::pb::waku::relay::v2::TopicDescriptor as TopicDescriptorProto;
}

pub mod pubsub {
    pub use super::pb::waku::relay::v2::rpc::SubOpts as SubOptsProto;
    pub use super::pb::waku::relay::v2::{
        ControlGraft as ControlGraftProto, ControlIHave as ControlIHaveProto, ControlIHave,
        ControlIWant as ControlIWantProto, ControlMessage as ControlMessageProto,
        ControlPrune as ControlPruneProto, Message as MessageProto, PeerInfo as PeerInfoProto,
        Rpc as FrameProto,
    };
}
