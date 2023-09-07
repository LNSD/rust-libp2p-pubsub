// @generated
// !Topic Descriptor
// !
// ! The topic descriptor message is used to define various options and parameters of a topic. It currently specifies
// ! the topic's human readable name, its authentication options, and its encryption options. The AuthOpts and EncOpts
// ! of the topic descriptor message are not used in current implementations, but may be used in future. For clarity,
// ! this is added as a comment in the file, and may be removed once used.
// !
// ! <https://github.com/libp2p/specs/blob/master/pubsub/README.md#the-topic-descriptor>

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TopicDescriptor {
    #[prost(string, optional, tag="1")]
    pub name: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(message, optional, tag="2")]
    pub auth: ::core::option::Option<topic_descriptor::AuthOpts>,
    #[prost(message, optional, tag="3")]
    pub enc: ::core::option::Option<topic_descriptor::EncOpts>,
}
/// Nested message and enum types in `TopicDescriptor`.
pub mod topic_descriptor {
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AuthOpts {
        #[prost(enumeration="auth_opts::AuthMode", optional, tag="1")]
        pub mode: ::core::option::Option<i32>,
        /// root keys to trust
        #[prost(bytes="bytes", repeated, tag="2")]
        pub keys: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
    }
    /// Nested message and enum types in `AuthOpts`.
    pub mod auth_opts {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
        #[repr(i32)]
        pub enum AuthMode {
            /// no authentication, anyone can publish
            None = 0,
            /// only messages signed by keys in the topic descriptor are accepted
            Key = 1,
            /// web of trust, certificates can allow publisher set to grow
            Wot = 2,
        }
        impl AuthMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    AuthMode::None => "NONE",
                    AuthMode::Key => "KEY",
                    AuthMode::Wot => "WOT",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "NONE" => Some(Self::None),
                    "KEY" => Some(Self::Key),
                    "WOT" => Some(Self::Wot),
                    _ => None,
                }
            }
        }
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
    pub struct EncOpts {
        #[prost(enumeration="enc_opts::EncMode", optional, tag="1")]
        pub mode: ::core::option::Option<i32>,
        /// the hashes of the shared keys used (salted)
        #[prost(bytes="bytes", repeated, tag="2")]
        pub key_hashes: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
    }
    /// Nested message and enum types in `EncOpts`.
    pub mod enc_opts {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
        #[repr(i32)]
        pub enum EncMode {
            /// no encryption, anyone can read
            None = 0,
            /// messages are encrypted with shared key
            Sharedkey = 1,
            /// web of trust, certificates can allow publisher set to grow
            Wot = 2,
        }
        impl EncMode {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    EncMode::None => "NONE",
                    EncMode::Sharedkey => "SHAREDKEY",
                    EncMode::Wot => "WOT",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "NONE" => Some(Self::None),
                    "SHAREDKEY" => Some(Self::Sharedkey),
                    "WOT" => Some(Self::Wot),
                    _ => None,
                }
            }
        }
    }
}
// @@protoc_insertion_point(module)
