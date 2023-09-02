#[cfg(any(feature = "prost_codec", feature = "quick_protobuf_codec"))]
pub mod codec;
pub mod heartbeat;
pub mod upgrade;

/// A stateful entity that can process and produce events.
pub mod service;
pub mod ttl_cache;
