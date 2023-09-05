use crate::upgrade::ProtocolUpgradeSend;

use super::router_trait::ProtocolRouter;

/// The pubsub protocol trait.
///
/// This trait is used by the [`Behaviour`](crate::behaviour::Behaviour) to identify the pubsub
/// protocol and create a connection handler instance for the protocol as well as the
/// [`ProtocolRouter`] instance responsible for routing messages to the appropriate peers.
pub trait Protocol {
    type Upgrade: ProtocolUpgradeSend + Clone;
    type RouterService: ProtocolRouter;

    /// Returns the protocol's upgrade.
    ///
    /// See [`ProtocolUpgrade`] for more information.
    fn upgrade() -> Self::Upgrade;

    /// Returns the protocol's router service.
    ///
    /// See [`ProtocolRouter`] for more information.
    fn router(&self) -> Self::RouterService;
}
