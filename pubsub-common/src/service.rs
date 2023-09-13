//! A service is a stateful entity that can receive events and produce events.
//!
//! The [`Service`](service::Service) trait is the main abstraction used by the this crate.
//! It is used to define the behavior of the different components of a libp2p protocol.
//!
//! Services are intended to be used wrapped in a [`ServiceContext`](service::ServiceContext)
//! implementation, which typically provides them with a mailbox for input events and a mailbox for
//! output events. Additionally, the wrapping [`ServiceContext`](service::ServiceContext) is in
//! charge of defining the polling strategy for the service.
//!
//! See [`Service`](service::Service) and [`ServiceContext`](service::ServiceContext) documentation
//! for more details.

pub use buffered_context::BufferedContext;
pub use context::ServiceContext;
pub use context_handles::{InCtx, JointCtx, OnEventCtx, OutCtx, PollCtx};
pub use event_handler::{wrap_handler, EventHandler, ServiceWrapper};
pub use service_trait::{InEvent, OutEvent, Service};

mod buffered_context;
mod context;
mod context_handles;
mod event_handler;
mod service_trait;
