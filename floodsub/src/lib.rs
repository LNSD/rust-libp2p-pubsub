pub use behaviour::{Behaviour, Event, PublishError, SendError, SubscriptionError};
pub use config::Config;
pub use frame::Message;
pub use topic::{Hasher, Topic, TopicHash};

mod behaviour;
mod config;
mod connections;
mod frame;
mod handler;
mod message_id;
mod proto;
mod router;
mod seqno;
mod topic;

pub type IdentTopic = Topic<topic::IdentityHash>;
pub type Sha256Topic = Topic<topic::Sha256Hash>;
