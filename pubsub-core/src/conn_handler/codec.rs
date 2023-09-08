#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use asynchronous_codec::{Decoder, Encoder};
use bytes::{Bytes, BytesMut};
use unsigned_varint::codec::UviBytes;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Maximum message length exceeded")]
    MaxMessageLenExceeded,

    #[error("Length-prefix error: {0}")]
    #[allow(clippy::enum_variant_names)]
    LengthPrefixError(std::io::Error),

    #[error(transparent)]
    #[allow(clippy::enum_variant_names)]
    IoError(#[from] std::io::Error),
}

/// Asynchronous codec implementation for the PubSub protocol that implements the [`Encoder`] and
/// [`Decoder`] traits from the [`asynchronous-codec`] crate to encode and decode
/// [`unsigned_varint`] length-prefixed frames.
pub struct Codec {
    uvi: UviBytes,
}

impl Codec {
    /// Create new [`Codec`].
    ///
    /// Parameter `max_message_len_bytes` determines the maximum length of the frame bytes. This
    /// limit does not take into account the length of the [`unsigned_varint`] encoded length
    /// prefix.
    pub fn new(max_message_len_bytes: usize) -> Self {
        let mut uvi = UviBytes::default();
        uvi.set_max_len(max_message_len_bytes);
        Self { uvi }
    }
}

impl Encoder for Codec {
    type Item = Bytes;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.uvi
            .encode(item, dst)
            .map_err(|_| Error::MaxMessageLenExceeded)
    }
}

impl Decoder for Codec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes = self.uvi.decode(src).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => Error::MaxMessageLenExceeded,
            std::io::ErrorKind::Other => Error::LengthPrefixError(e),
            _ => unreachable!("Unexpected error kind: {:?}", e.kind()),
        })?;
        Ok(bytes.map(|b| b.freeze()))
    }
}
