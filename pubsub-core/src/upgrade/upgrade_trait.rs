use libp2p::core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use libp2p::swarm::handler::{InboundUpgradeSend, OutboundUpgradeSend, UpgradeInfoSend};
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
        + ProtocolOutboundUpgrade<<Self as UpgradeInfo>::Info>;


    /// Implemented automatically on all types that implement [`ProtocolUpgrade`]
    /// and `Send + 'static`.
    ///
    /// Do not implement this trait yourself. Instead, please implement
    /// [`ProtocolUpgrade`] sub-traits.
    pub trait ProtocolUpgradeSend = UpgradeInfoSend +
        InboundUpgradeSend<Output=ProtocolUpgradeOutput<<Self as UpgradeInfoSend>::Info>> +
        OutboundUpgradeSend<Output=ProtocolUpgradeOutput<<Self as UpgradeInfoSend>::Info>>;
}
