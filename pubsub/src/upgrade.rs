pub use simple::SimpleProtocolUpgrade;
pub use upgrade_trait::{
    ProtocolInboundUpgrade, ProtocolOutboundUpgrade, ProtocolUpgrade, ProtocolUpgradeInfo,
    ProtocolUpgradeOutput,
};

mod simple;
mod upgrade_trait;
