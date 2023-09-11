pub use events::{
    MessageEvent as MessageIdMessageEvent, ServiceIn as MessageIdInEvent,
    ServiceOut as MessageIdOutEvent, SubscriptionEvent as MessageIdSubscriptionEvent,
};
pub use service::MessageIdService;

mod events;
mod service;

#[cfg(test)]
mod tests;
