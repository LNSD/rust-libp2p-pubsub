pub use topic::{Hasher, IdentityHash, Sha256Hash, Topic, TopicHash};

mod framing;
mod topic;

pub type IdentTopic = Topic<IdentityHash>;
pub type Sha256Topic = Topic<Sha256Hash>;
