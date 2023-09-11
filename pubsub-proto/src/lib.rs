mod gen {
    include!("gen/mod.rs");
}

#[doc = include_str!("./gen/docs/libp2p/topic_descriptor/v1/docs.md")]
#[allow(rustdoc::invalid_html_tags)]
pub mod topic_descriptor {
    pub use super::gen::libp2p::topic_descriptor::v1::TopicDescriptor as TopicDescriptorProto;
}

#[doc = include_str!("./gen/docs/libp2p/pubsub/v1/docs.md")]
#[allow(rustdoc::invalid_html_tags)]
pub mod pubsub {
    pub use super::gen::libp2p::pubsub::v1::{
        ControlGraft as ControlGraftProto, ControlIHave as ControlIHaveProto, ControlIHave,
        ControlIWant as ControlIWantProto, ControlMessage as ControlMessageProto,
        ControlPrune as ControlPruneProto, Frame as FrameProto, Message as MessageProto,
        PeerInfo as PeerInfoProto, SubOpts as SubOptsProto,
    };
}
