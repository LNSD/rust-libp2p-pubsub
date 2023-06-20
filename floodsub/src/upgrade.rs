use std::convert::Infallible;
use std::future::Future;
use std::iter;
use std::pin::Pin;

use asynchronous_codec::Framed;
use futures::{future, AsyncRead, AsyncWrite};
use libp2p::core::UpgradeInfo;
use libp2p::{InboundUpgrade, OutboundUpgrade};

use common::prost_protobuf_codec::Codec as ProstCodec;

use crate::proto::RpcProto;
use crate::protocol_id::{SingleProtocolId, PROTOCOL_ID};

type Codec = ProstCodec<RpcProto>;
type ProtocolId = SingleProtocolId<&'static [u8]>;

#[derive(Debug, Clone)]
pub struct ProtocolUpgrade {
    protocol_id: ProtocolId,
    max_frame_size: usize,
}

impl ProtocolUpgrade {
    pub fn new(max_frame_size: usize) -> Self {
        Self {
            protocol_id: ProtocolId::new(PROTOCOL_ID),
            max_frame_size,
        }
    }
}

impl UpgradeInfo for ProtocolUpgrade {
    type Info = ProtocolId;
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(self.protocol_id.clone())
    }
}

impl<TSocket> InboundUpgrade<TSocket> for ProtocolUpgrade
where
    TSocket: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = (Framed<TSocket, Codec>, ProtocolId);
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: TSocket, info: Self::Info) -> Self::Future {
        Box::pin(future::ok((
            Framed::new(socket, Codec::new(self.max_frame_size)),
            info,
        )))
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for ProtocolUpgrade
where
    TSocket: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = (Framed<TSocket, Codec>, ProtocolId);
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, socket: TSocket, info: Self::Info) -> Self::Future {
        Box::pin(future::ok((
            Framed::new(socket, Codec::new(self.max_frame_size)),
            info,
        )))
    }
}
