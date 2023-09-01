pub use behaviour::Behaviour;
pub use config::Config;
pub use event::{Event, Message};
pub use message_id::{default_message_id_fn, MessageId, MessageIdFn};
pub use protocol::{
    Protocol, ProtocolId, ProtocolRouter, ProtocolRouterConnectionEvent, ProtocolRouterInEvent,
    ProtocolRouterOutEvent, ProtocolRouterSubscriptionEvent,
};
pub use subscription::{Subscription, SubscriptionBuilder};
pub use topic::{Hasher, IdentityHash, Sha256Hash, Topic, TopicHash};

mod behaviour;
mod config;
mod conn_handler;
mod event;
mod framing;
mod message_id;
mod protocol;
mod services;
mod subscription;
mod topic;

pub type IdentTopic = Topic<IdentityHash>;
pub type Sha256Topic = Topic<Sha256Hash>;
