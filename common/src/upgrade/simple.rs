use std::convert::Infallible;
use std::iter;

use futures::{future, AsyncRead, AsyncWrite};
use libp2p::core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo};

/// Output of the [`SimpleUpgrade`] upgrade.
pub struct SimpleUpgradeOutput<TInfo, TSocket> {
    pub socket: TSocket,
    pub info: TInfo,
}

/// A protocol upgrade implementation that just returns the socket and the upgrade protocol info.
#[derive(Debug, Clone)]
pub struct SimpleUpgrade<TInfo> {
    protocol_info: TInfo,
}

impl<TInfo> SimpleUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone,
{
    pub fn new(info: TInfo) -> Self {
        Self {
            protocol_info: info,
        }
    }
}

impl<TInfo> UpgradeInfo for SimpleUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone,
{
    type Info = TInfo;
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(self.protocol_info.clone())
    }
}

impl<TInfo, TSocket> InboundUpgrade<TSocket> for SimpleUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone,
    TSocket: AsyncRead + AsyncWrite + Unpin,
{
    type Output = SimpleUpgradeOutput<TInfo, TSocket>;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: TSocket, info: Self::Info) -> Self::Future {
        future::ok(SimpleUpgradeOutput { socket, info })
    }
}

impl<TInfo, TSocket> OutboundUpgrade<TSocket> for SimpleUpgrade<TInfo>
where
    TInfo: AsRef<str> + Clone,
    TSocket: AsyncRead + AsyncWrite + Unpin,
{
    type Output = SimpleUpgradeOutput<TInfo, TSocket>;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: TSocket, info: Self::Info) -> Self::Future {
        future::ok(SimpleUpgradeOutput { socket, info })
    }
}
