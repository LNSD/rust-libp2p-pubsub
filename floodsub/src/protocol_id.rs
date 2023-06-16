use libp2p::core::ProtocolName;

pub const PROTOCOL_ID: &[u8] = b"/floodsub/1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleProtocolId<N: AsRef<[u8]>>(N);

impl<N> SingleProtocolId<N>
where
    N: AsRef<[u8]>,
{
    pub fn new(protocol_id: N) -> Self {
        Self(protocol_id)
    }
}

impl<N> ProtocolName for SingleProtocolId<N>
where
    N: AsRef<[u8]>,
{
    fn protocol_name(&self) -> &[u8] {
        self.0.as_ref()
    }
}
