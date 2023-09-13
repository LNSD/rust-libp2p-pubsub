use libp2p_pubsub_common::service::{EventHandler, OnEventCtx};
use libp2p_pubsub_core::protocol::{Protocol, ProtocolRouterInEvent, ProtocolRouterOutEvent};
use libp2p_pubsub_core::upgrade::SimpleProtocolUpgrade;

/// The protocol ID for the noop protocol.
pub const NOOP_PROTOCOL_ID: &str = "/noop/1.0.0";

/// The noop protocol is a dummy protocol implementation for testing purposes.
#[derive(Default)]
pub struct NoopProtocol;

impl Protocol for NoopProtocol {
    type Upgrade = SimpleProtocolUpgrade<&'static str>;
    type RouterService = NoopProtocolRouter;

    fn upgrade() -> Self::Upgrade {
        SimpleProtocolUpgrade::new(NOOP_PROTOCOL_ID)
    }

    fn router(&self) -> Self::RouterService {
        Default::default()
    }
}

/// The pubsub protocol router service for the noop protocol.
#[derive(Default)]
pub struct NoopProtocolRouter;

impl EventHandler for NoopProtocolRouter {
    type InEvent = ProtocolRouterInEvent;
    type OutEvent = ProtocolRouterOutEvent;

    fn on_event<'a>(
        &mut self,
        _svc_cx: &mut impl OnEventCtx<'a, Self::OutEvent>,
        _ev: Self::InEvent,
    ) {
        // No-op
    }
}
