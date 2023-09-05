use pubsub::upgrade::SimpleProtocolUpgrade;

use crate::router::Router;

/// Floodsub Protocol ID string.
pub const PROTOCOL_ID: &str = "/floodsub/1.0.0";

/// The Floodsub pubsub protocol.
#[derive(Default)]
pub struct Protocol;

impl pubsub::protocol::Protocol for Protocol {
    type Upgrade = SimpleProtocolUpgrade<&'static str>;
    type RouterService = Router;

    fn upgrade() -> Self::Upgrade {
        SimpleProtocolUpgrade::new(PROTOCOL_ID)
    }

    fn router(&self) -> Self::RouterService {
        Default::default()
    }
}
