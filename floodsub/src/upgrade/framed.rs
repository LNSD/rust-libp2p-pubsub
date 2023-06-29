use std::convert::Infallible;
use std::iter;

use futures::{future, AsyncRead, AsyncWrite};
use libp2p::core::UpgradeInfo;
use libp2p::{InboundUpgrade, OutboundUpgrade};

use common::codec::{Framed, ProstCodec};

use crate::proto::RpcProto;

type Codec = ProstCodec<RpcProto>;

pub struct FramedUpgradeOutput<S, P> {
    pub stream: Framed<S, Codec>,
    pub info: P,
}

#[derive(Debug, Clone)]
pub struct FramedUpgrade<P> {
    protocol_id: P,
    max_frame_size: usize,
}

impl<P> FramedUpgrade<P>
where
    P: AsRef<str> + Clone,
{
    pub fn new(protocol_id: P, max_frame_size: usize) -> Self {
        Self {
            protocol_id,
            max_frame_size,
        }
    }
}

impl<P> UpgradeInfo for FramedUpgrade<P>
where
    P: AsRef<str> + Clone,
{
    type Info = P;
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(self.protocol_id.clone())
    }
}

impl<S, P> InboundUpgrade<S> for FramedUpgrade<P>
where
    S: AsyncRead + AsyncWrite + Unpin,
    P: AsRef<str> + Clone,
{
    type Output = FramedUpgradeOutput<S, P>;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: S, info: Self::Info) -> Self::Future {
        let codec = Codec::new(self.max_frame_size);
        future::ok(FramedUpgradeOutput {
            stream: Framed::new(socket, codec),
            info,
        })
    }
}

impl<S, P> OutboundUpgrade<S> for FramedUpgrade<P>
where
    S: AsyncRead + AsyncWrite + Unpin,
    P: AsRef<str> + Clone,
{
    type Output = FramedUpgradeOutput<S, P>;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: S, info: Self::Info) -> Self::Future {
        let codec = Codec::new(self.max_frame_size);
        future::ok(FramedUpgradeOutput {
            stream: Framed::new(socket, codec),
            info,
        })
    }
}
