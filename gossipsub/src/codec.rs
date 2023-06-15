use std::io;

use asynchronous_codec::{Decoder, Encoder};
use bytes::BytesMut;

use common::prost_protobuf_codec::Codec as ProstCodec;

use crate::handler::HandlerEvent;
use crate::rpc::RpcProto;

pub struct Codec {
    /// The codec to handle common encoding/decoding of protobuf messages
    codec: ProstCodec<RpcProto>,
}

impl Codec {
    pub fn new(max_len_bytes: usize) -> Self {
        let codec = ProstCodec::new(max_len_bytes);
        Self { codec }
    }
}

impl Encoder for Codec {
    type Item = RpcProto;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.codec.encode(item, dst)
    }
}

impl Decoder for Codec {
    type Item = HandlerEvent;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.codec.decode(src).map(|rpc| rpc.map(HandlerEvent::Rpc))
    }
}
