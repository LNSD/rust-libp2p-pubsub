pub use protocol_trait::{Protocol, ProtocolId};
pub use router_trait::{
    ProtocolRouter, ProtocolRouterConnectionEvent, ProtocolRouterInEvent, ProtocolRouterOutEvent,
    ProtocolRouterSubscriptionEvent,
};

mod protocol_trait;
mod router_trait;
