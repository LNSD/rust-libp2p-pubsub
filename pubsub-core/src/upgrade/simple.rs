use std::convert::Infallible;
use std::iter;

use futures::future;
use libp2p::core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use libp2p::swarm::Stream;

use super::upgrade_trait::ProtocolUpgradeOutput;

/// A simple [`ProtocolUpgrade`](super::upgrade_trait::ProtocolUpgrade) implementation that just
/// returns the socket and the protocol upgrade infos
#[derive(Debug, Clone)]
pub struct SimpleProtocolUpgrade<TInfo> {
    protocol_info: TInfo,
}

impl<TInfo> SimpleProtocolUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone + Send + 'static,
{
    pub fn new(protocol_info: TInfo) -> Self {
        Self { protocol_info }
    }
}

impl<TInfo> UpgradeInfo for SimpleProtocolUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone + Send + 'static,
{
    type Info = TInfo;
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(self.protocol_info.clone())
    }
}

impl<TInfo> InboundUpgrade<Stream> for SimpleProtocolUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone + Send + 'static,
{
    type Output = ProtocolUpgradeOutput<TInfo>;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: Stream, info: Self::Info) -> Self::Future {
        future::ok(ProtocolUpgradeOutput { socket, info })
    }
}

impl<TInfo> OutboundUpgrade<Stream> for SimpleProtocolUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone + Send + 'static,
{
    type Output = ProtocolUpgradeOutput<TInfo>;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: Stream, info: Self::Info) -> Self::Future {
        future::ok(ProtocolUpgradeOutput { socket, info })
    }
}
