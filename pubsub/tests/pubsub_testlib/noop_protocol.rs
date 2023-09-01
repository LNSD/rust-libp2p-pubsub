use common::service::Service;
use pubsub::{Protocol, ProtocolId, ProtocolRouterInEvent, ProtocolRouterOutEvent};

/// The protocol ID for the noop protocol.
pub const NOOP_PROTOCOL_ID: &str = "/noop/1.0.0";

/// The noop protocol is a dummy protocol implementation for testing purposes.
#[derive(Default)]
pub struct NoopProtocol;

impl Protocol for NoopProtocol {
    type ProtocolId = NoopProtocolId;
    type RouterService = NoopProtocolRouter;

    fn router(&self) -> Self::RouterService {
        Default::default()
    }
}

/// The protocol ID for the noop protocol.
pub struct NoopProtocolId;

impl ProtocolId for NoopProtocolId {
    const PROTOCOL_ID: &'static str = NOOP_PROTOCOL_ID;
}

/// The pubsub protocol router service for the noop protocol.
#[derive(Default)]
pub struct NoopProtocolRouter;

impl Service for NoopProtocolRouter {
    type InEvent = ProtocolRouterInEvent;
    type OutEvent = ProtocolRouterOutEvent;

    fn on_event(&mut self, _ev: Self::InEvent) -> Option<Self::OutEvent> {
        None
    }
}
