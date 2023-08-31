pub use pb::waku::relay::v2::rpc::SubOpts as SubOptsProto;
pub use pb::waku::relay::v2::{
    ControlGraft as ControlGraftProto, ControlIHave as ControlIHaveProto, ControlIHave,
    ControlIWant as ControlIWantProto, ControlMessage as ControlMessageProto,
    ControlPrune as ControlPruneProto, Message as MessageProto, PeerInfo as PeerInfoProto,
    Rpc as FrameProto, TopicDescriptor as TopicDescriptorProto,
};

mod pb {
    include!(concat!(env!("OUT_DIR"), "/proto/mod.rs"));
}
