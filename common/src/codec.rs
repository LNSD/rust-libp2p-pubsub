pub use asynchronous_codec::{Framed, FramedRead, FramedWrite};

#[cfg(feature = "prost_codec")]
pub use self::prost_protobuf::{Codec as ProstCodec, Error as ProstCodecError};
#[cfg(feature = "quick_protobuf_codec")]
pub use self::quick_protobuf::{Codec as QuickProtobufCodec, Error as QuickProtobufCodecError};

#[cfg(feature = "prost_codec")]
mod prost_protobuf;
#[cfg(feature = "quick_protobuf_codec")]
mod quick_protobuf;
