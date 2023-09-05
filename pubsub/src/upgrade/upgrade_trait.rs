use libp2p::core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use libp2p::swarm::Stream;

/// Output of the [`InboundUpgrade`] and [`OutboundUpgrade`] traits.
pub struct ProtocolUpgradeOutput<TInfo> {
    pub socket: Stream,
    pub info: TInfo,
}

// NOTE: Use `trait_set` crate as `trait_alias` is not yet stable.
//       https://github.com/rust-lang/rust/issues/41517
trait_set::trait_set! {
    pub trait ProtocolUpgradeInfo = UpgradeInfo;
    pub trait ProtocolInboundUpgrade<TInfo> = InboundUpgrade<Stream, Output=ProtocolUpgradeOutput<TInfo>>;
    pub trait ProtocolOutboundUpgrade<TInfo> = OutboundUpgrade<Stream, Output=ProtocolUpgradeOutput<TInfo>>;

    /// The `ProtocolUpgrade` trait is an alias for the [`InboundUpgrade`], [`OutboundUpgrade`] and
    /// [`UpgradeInfo`] traits.
    ///
    /// Theses traits are used by the connection handler to identify and negotiate the pubsub
    /// protocol.
    ///
    /// - The [`UpgradeInfo`] trait describes the protocol name and version.
    /// - The [`InboundUpgrade`] trait describes the protocol handshake to apply on inbound
    ///   connections.
    /// - The [`OutboundUpgrade`] trait describes the protocol handshake to apply on outbound
    ///   connections.
    pub trait ProtocolUpgrade = ProtocolUpgradeInfo
        + ProtocolInboundUpgrade<<Self as UpgradeInfo>::Info>
        + ProtocolOutboundUpgrade<<Self as UpgradeInfo>::Info>
        + Clone;
}
