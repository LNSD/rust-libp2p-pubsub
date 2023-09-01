use crate::router::Router;

/// Floodsub Protocol ID string.
const FLOODSUB_PROTOCOL_ID: &str = "/floodsub/1.0.0";

/// The Floodsub protocol.
#[derive(Default)]
pub struct Protocol;

impl pubsub::Protocol for Protocol {
    type ProtocolId = ProtocolId;
    type RouterService = Router;

    fn router(&self) -> Self::RouterService {
        Default::default()
    }
}

/// The Floodsub protocol ID.
#[derive(Default)]
pub struct ProtocolId;

impl pubsub::ProtocolId for ProtocolId {
    const PROTOCOL_ID: &'static str = FLOODSUB_PROTOCOL_ID;
}
