pub const PROTOCOL_ID: &str = "/floodsub/1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticProtocolId<N: AsRef<str>>(N);

impl<N> StaticProtocolId<N>
where
    N: AsRef<str>,
{
    pub fn new(protocol_id: N) -> Self {
        Self(protocol_id)
    }
}

impl<N> AsRef<str> for StaticProtocolId<N>
where
    N: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
