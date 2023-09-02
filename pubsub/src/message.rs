//! The `Message` type represents a bit of data which is published to a pubsub topic, distributed
//! over a pubsub network and received by any node subscribed to the topic.
//!
//! This module contains the public API type of a pubsub message.

// TODO: Do not re-export framing message type.
pub use crate::framing::Message;
