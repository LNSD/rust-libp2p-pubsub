use common::service::{OnEventCtx, Service};
use pubsub::protocol::{Protocol, ProtocolRouterInEvent, ProtocolRouterOutEvent};
use pubsub::upgrade::SimpleProtocolUpgrade;

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

impl Service for NoopProtocolRouter {
    type InEvent = ProtocolRouterInEvent;
    type OutEvent = ProtocolRouterOutEvent;

    fn on_event(&mut self, _svc_cx: &mut OnEventCtx<'_, Self::OutEvent>, _ev: Self::InEvent) {
        // No-op
    }
}
