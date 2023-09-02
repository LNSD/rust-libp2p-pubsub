//! A service is a stateful entity that can receive events and produce events.
//!
//! The [`Service`](service::Service) trait is the main abstraction used by the this crate.
//! It is used to define the behavior of the different components of a libp2p protocol.
//!
//! Services are intended to be used wrapped in a [`Context`](service::Context), which provides the
//! with a mailbox for input events and a mailbox for output events. The
//! [`Context`](service::Context) is in charge of polling the service for events and processing the
//! mailbox events.
//!
//! See [`Service`](service::Service) and [`Context`](service::Context) documentation for more
//! details.

pub use context::Context;
pub use context_handles::{OnEventCtx, PollCtx};
pub use service_trait::{InEvent, OutEvent, Service};

mod context;
mod context_handles;
mod service_trait;
