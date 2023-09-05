use super::router_trait::ProtocolRouter;

/// The pubsub protocol trait.
///
/// This trait is used by the [`Behaviour`](crate::behaviour::Behaviour) to identify the pubsub
/// protocol and create a connection handler instance for the protocol as well as the
/// [`ProtocolRouter`] instance responsible for routing messages to the appropriate peers.
pub trait Protocol: 'static {
    type ProtocolId: ProtocolId;
    type RouterService: ProtocolRouter;

    /// Returns the protocol's ID string.
    ///
    /// See [`ProtocolId`] for more information.
    // TODO: Revisit this after refactoring the Pubsub connection handler.
    fn protocol_id() -> &'static str {
        Self::ProtocolId::PROTOCOL_ID
    }

    /// Returns the protocol's router service.
    ///
    /// See [`ProtocolRouter`] for more information.
    fn router(&self) -> Self::RouterService;
}

/// The protocol id trait.
///
/// This trait is used by the [`Behaviour`](crate::behaviour::Behaviour) to identify the pubsub
/// protocol and create a connection handler instance for the protocol.
pub trait ProtocolId {
    // TODO: Revisit this after refactoring the Pubsub connection handler.
    const PROTOCOL_ID: &'static str;
}
