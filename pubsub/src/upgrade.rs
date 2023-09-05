pub use simple::SimpleProtocolUpgrade;
pub use upgrade_trait::{
    ProtocolInboundUpgrade, ProtocolOutboundUpgrade, ProtocolUpgrade, ProtocolUpgradeInfo,
    ProtocolUpgradeOutput, ProtocolUpgradeSend,
};

mod simple;
mod upgrade_trait;
