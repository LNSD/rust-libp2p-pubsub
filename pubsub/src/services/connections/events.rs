use libp2p::core::ConnectedPoint;
use libp2p::identity::PeerId;
use libp2p::swarm::ConnectionId;
use libp2p::Multiaddr;

/// The events emitted by libp2p's [`Swarm`](libp2p::swarm::Swarm)'s connection handling logic.
#[derive(Debug, Clone)]
pub enum ServiceIn {
    /// Event emitted by the NetworkBehaviour's [`handle_established_inbound_connection`](libp2p::swarm::NetworkBehaviour::handle_established_inbound_connection) callback method:
    ///
    /// >  Callback that is invoked for every new inbound connection.
    /// >
    /// >  At this point in the connection lifecycle, only the remote's and our local address are known.
    /// >  We have also already allocated a [`ConnectionId`].
    /// >
    /// >  Any error returned from this function will immediately abort the dial attempt.
    EstablishedInboundConnection {
        connection_id: ConnectionId,
        peer_id: PeerId,
        local_addr: Multiaddr,
        remote_addr: Multiaddr,
    },
    /// Event emitted by the NetworkBehaviour's [`handle_established_outbound_connection`](libp2p::swarm::NetworkBehaviour::handle_established_outbound_connection) callback method.
    ///
    /// > Callback that is invoked for every established outbound connection.
    /// >
    /// > This is invoked once we have successfully dialed a peer.
    /// > At this point, we have verified their [`PeerId`] and we know, which particular [`Multiaddr`] succeeded in the dial.
    /// > In order to actually use this connection, this function must return a [`ConnectionHandler`](crate::ConnectionHandler).
    /// >
    /// > Returning an error will immediately close the connection.
    EstablishedOutboundConnection {
        connection_id: ConnectionId,
        peer_id: PeerId,
        remote_addr: Multiaddr,
    },
    /// Inform the behaviour that a connection event, coming from the swarm, happened.
    SwarmEvent(SwarmEvent),
}

impl ServiceIn {
    /// Create a new [`ServiceIn::SwarmEvent`] event.
    pub fn from_swarm_event(event: impl Into<SwarmEvent>) -> Self {
        Self::SwarmEvent(event.into())
    }
}

/// The events emitted by libp2p's [`Swarm`](libp2p::swarm::Swarm)'s connection handling logic.
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Informs the behaviour that the dial to a known or unknown peer failed.
    ///
    /// This event maps to NetworkBehaviour's [`FromSwarm::DialFailure`](libp2p::swarm::behaviour::FromSwarm::DialFailure) event.
    DialFailure {
        connection_id: ConnectionId,
        peer_id: Option<PeerId>,
        error: String, // TODO: Revisit what error type to use here
    },
    /// Informs the behaviour that an error  happened on an incoming connection during its initial handshake. This can include,
    /// for example, an error during the handshake of the encryption layer, or the connection unexpectedly closed.
    ///
    /// This event maps to NetworkBehaviour's [`FromSwarm::ListenFailure`](libp2p::swarm::behaviour::FromSwarm::ListenFailure) event.
    ListenFailure {
        connection_id: ConnectionId,
        local_addr: Multiaddr,
        send_back_addr: Multiaddr,
        error: String, // TODO: Revisit what error type to use here
    },
    /// Informs the behaviour about a newly established connection to a peer.
    ///
    /// This event maps to NetworkBehaviour's [`FromSwarm::ConnectionEstablished`](libp2p::swarm::behaviour::FromSwarm::ConnectionEstablished) event.
    ConnectionEstablished {
        connection_id: ConnectionId,
        peer_id: PeerId,
    },
    /// Informs the behaviour about a closed connection to a peer.
    ///
    /// This event is always paired with an earlier [`ServiceIn::ConnectionEstablished`] with the same peer ID, connection ID
    /// and endpoint.
    ///
    /// This event maps to NetworkBehaviour's [`FromSwarm::ConnectionClosed`](libp2p::swarm::behaviour::FromSwarm::ConnectionClosed) event.
    ConnectionClosed {
        connection_id: ConnectionId,
        peer_id: PeerId,
    },
    /// Informs the behaviour that the [`ConnectedPoint`] of an existing connection has changed.
    ///
    /// This event maps to NetworkBehaviour's [`FromSwarm::AddressChange`](libp2p::swarm::behaviour::FromSwarm::AddressChange) event.
    ConnectionAddressChange {
        connection_id: ConnectionId,
        peer_id: PeerId,
        old: ConnectedPoint,
        new: ConnectedPoint,
    },
}

/// The events emitted by the [`ConnectionsService`].
#[derive(Debug, Clone)]
pub enum ServiceOut {
    /// This event is emitted when a new connection is established the first time we connect to a
    /// peer.
    ///
    /// As peers are removed from the connection service when they are disconnected, when a
    /// previously disconnected peer is reconnected, this event will be emitted again.
    NewPeerConnected(PeerId),
    /// This event is emitted when all connections to a peer are closed. In this case the peer is
    /// removed from the connection service.
    PeerDisconnected(PeerId),
}
