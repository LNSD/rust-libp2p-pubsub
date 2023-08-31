use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use base64::prelude::*;
use prost::Message;
use sha2::{Digest, Sha256};

use crate::framing::TopicDescriptorProto;

/// A generic trait that can be extended for various hashing frame for a topic.
pub trait Hasher {
    /// The function that takes a topic string and creates a topic hash.
    fn hash(topic: String) -> TopicHash;
}

/// A type for representing topics who use the identity hash.
#[derive(Debug, Clone)]
pub struct IdentityHash;

impl Hasher for IdentityHash {
    /// Creates a [`TopicHash`] as a raw string.
    fn hash(topic_string: String) -> TopicHash {
        TopicHash { hash: topic_string }
    }
}

#[derive(Debug, Clone)]
pub struct Sha256Hash;

impl Hasher for Sha256Hash {
    /// Creates a [`TopicHash`] by SHA256 hashing the topic then base64 encoding the
    /// hash.
    fn hash(topic_string: String) -> TopicHash {
        let topic_descriptor = TopicDescriptorProto {
            name: Some(topic_string),
            auth: None,
            enc: None,
        };
        let mut bytes = Vec::with_capacity(topic_descriptor.encoded_len());
        topic_descriptor
            .encode(&mut bytes)
            .expect("Encoding to succeed");
        let hash = BASE64_STANDARD.encode(Sha256::digest(&bytes));
        TopicHash { hash }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TopicHash {
    /// The topic hash. Stored as a string to align with the protobuf API.
    hash: String,
}

impl TopicHash {
    pub fn from_raw<T: Into<String>>(raw: T) -> Self {
        Self { hash: raw.into() }
    }

    pub fn into_string(self) -> String {
        self.hash
    }

    pub fn as_str(&self) -> &str {
        &self.hash
    }
}

impl<T: Into<String>> From<T> for TopicHash {
    fn from(hash: T) -> Self {
        Self::from_raw(hash)
    }
}

impl FromStr for TopicHash {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_raw(s))
    }
}

impl AsRef<str> for TopicHash {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A pub-sub topic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Topic<H: Hasher> {
    topic: String,
    phantom_data: std::marker::PhantomData<H>,
}

impl<H: Hasher> From<Topic<H>> for TopicHash {
    fn from(topic: Topic<H>) -> TopicHash {
        topic.hash()
    }
}

impl<H: Hasher> Topic<H> {
    pub fn new<T: Into<String>>(topic: T) -> Self {
        Topic {
            topic: topic.into(),
            phantom_data: std::marker::PhantomData,
        }
    }

    pub fn hash(&self) -> TopicHash {
        H::hash(self.topic.clone())
    }
}

impl<H: Hasher> fmt::Display for Topic<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.topic)
    }
}

impl fmt::Display for TopicHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hash)
    }
}
