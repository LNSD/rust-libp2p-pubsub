use std::collections::HashMap;

use libp2p::identity::PeerId;
use libp2p::swarm::ConnectionId;
use libp2p::Multiaddr;

use common::service::Service;

use crate::services::connections::events::SwarmEvent;

use super::connection::{Connection, ConnectionState};
use super::events::{ServiceIn, ServiceOut};

/// Manages the connections of the floodsub protocol behaviour.
#[derive(Debug, Default)]
pub struct ConnectionsService {
    /// The connection state of each connection.
    connections: HashMap<ConnectionId, Connection>,

    /// This table keeps track of the connections for each peer. It includes all connections,
    /// including those that are not yet established.
    ///
    /// Mapping `PeerId` to connection `ConnectionID`s.
    peer_connections: HashMap<PeerId, Vec<ConnectionId>>,

    /// This table keeps track of the established connections for each peer.
    ///
    /// Mapping `PeerId` to established connection `ConnectionID`s.
    peer_active_connections: HashMap<PeerId, Vec<ConnectionId>>,
}

// Private API.
impl ConnectionsService {
    fn register_connection(
        &mut self,
        peer: PeerId,
        connection_id: ConnectionId,
        connection: Connection,
    ) {
        // Insert the connection into the peer established connections map, if is established and
        // it doesn't exist yet.
        if connection.is_established() {
            let entry = self.peer_active_connections.entry(peer).or_default();
            if !entry.contains(&connection_id) {
                entry.push(connection_id);
            }
        }

        // Insert the connection into the peer connections map, if it doesn't exist yet.
        let entry = self.peer_connections.entry(peer).or_default();
        if !entry.contains(&connection_id) {
            entry.push(connection_id);
        }

        // Insert the connection into the connections map.
        self.connections.insert(connection_id, connection);
    }

    fn deregister_connection(&mut self, peer: &PeerId, connection: &ConnectionId) {
        // Remove the connection from the peer established connections map. If no more established
        // connections exist for the peer, remove the peer from the map.
        if let Some(conns) = self.peer_active_connections.get_mut(peer) {
            conns.retain(|id| id != connection);
            if conns.is_empty() {
                self.peer_active_connections.remove(peer);
            }
        }

        // Remove the connection from the peer connections map. If no more connections exist for the
        // peer, remove the peer from the map.
        if let Some(conns) = self.peer_connections.get_mut(peer) {
            conns.retain(|id| id != connection);
            if conns.is_empty() {
                self.peer_connections.remove(peer);
            }
        }

        // Remove the connection from the connections map.
        self.connections.remove(connection);
    }

    /// Register a new inbound connection with the given peer.
    ///
    /// The connection is registered with the given connection ID and the given local and remote
    /// addresses. The connection is registered with the `ConnectionState::Connecting` state.
    fn register_inbound(
        &mut self,
        connection: ConnectionId,
        peer: PeerId,
        local_addr: Multiaddr,
        remote_addr: Multiaddr,
    ) {
        let conn = Connection::new_inbound(local_addr, remote_addr);
        self.register_connection(peer, connection, conn);
    }

    /// Register a new outbound connection with the given peer.
    ///
    /// The connection is registered with the given connection ID and the given remote address. The
    /// connection is registered with the `ConnectionState::Connecting` state.
    fn register_outbound(
        &mut self,
        connection: ConnectionId,
        peer: PeerId,
        remote_addr: Multiaddr,
    ) {
        let conn = Connection::new_outbound(remote_addr);
        self.register_connection(peer, connection, conn);
    }

    /// Update the connection state of the connection with the given ID. It is a no-op if the
    /// connection does not exist.
    ///
    /// If the new state is `ConnectionState::Established`, the connection is also added to the
    /// `peer_established_connections` table.
    fn update_connection_state(
        &mut self,
        peer: &PeerId,
        connection: &ConnectionId,
        state: ConnectionState,
    ) {
        if let Some(conn) = self.connections.get_mut(connection) {
            conn.set_state(state);
        } else {
            return;
        }

        if state == ConnectionState::Established {
            self.peer_active_connections
                .entry(*peer)
                .or_default()
                .push(*connection);
        }
    }

    /// Update the connection state of the connection with the given ID. It is a no-op if the
    /// connection does not exist.
    fn update_connection_remote_address(
        &mut self,
        connection: &ConnectionId,
        remote_addr: Multiaddr,
    ) {
        if let Some(conn) = self.connections.get_mut(connection) {
            conn.set_remote_address(remote_addr);
        }
    }
}

/// Public API.
impl ConnectionsService {
    /// Returns the number of connections established with the given peer.
    #[must_use]
    pub fn peer_connections_count(&self, peer: &PeerId) -> usize {
        self.peer_active_connections
            .get(peer)
            .map_or(0, |v| v.len())
    }

    /// Get a list of all peers with at least one established connections.
    #[must_use]
    pub fn active_peers(&self) -> Vec<PeerId> {
        self.peer_active_connections
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    }

    /// Get then number of peers with at least one connection in established state.
    #[must_use]
    pub fn active_peers_count(&self) -> usize {
        self.peer_active_connections.len()
    }
}

impl Service for ConnectionsService {
    type InEvent = ServiceIn;
    type OutEvent = ServiceOut;

    #[tracing::instrument(skip_all)]
    fn on_event(&mut self, ev: Self::InEvent) -> Option<Self::OutEvent> {
        match ev {
            ServiceIn::EstablishedInboundConnection {
                connection_id,
                peer_id,
                local_addr,
                remote_addr,
            } => {
                tracing::trace!(peer = %peer_id, "Established inbound connection");
                self.register_inbound(
                    connection_id,
                    peer_id,
                    local_addr.clone(),
                    remote_addr.clone(),
                );
            }
            ServiceIn::EstablishedOutboundConnection {
                connection_id,
                peer_id,
                remote_addr,
            } => {
                tracing::trace!(peer = %peer_id, "Established outbound connection");
                self.register_outbound(connection_id, peer_id, remote_addr.clone());
            }
            ServiceIn::SwarmEvent(swarm_ev) => match swarm_ev {
                SwarmEvent::ConnectionEstablished {
                    connection_id,
                    peer_id,
                } => {
                    tracing::trace!(peer = %peer_id, "Connection established");
                    self.update_connection_state(
                        &peer_id,
                        &connection_id,
                        ConnectionState::Established,
                    );

                    // If this is the first connection with the peer, emit a `NewPeerConnected` event.
                    if self.peer_connections_count(&peer_id) == 1 {
                        return Some(ServiceOut::NewPeerConnected(peer_id));
                    }
                }
                SwarmEvent::ConnectionClosed {
                    connection_id,
                    peer_id,
                } => {
                    tracing::trace!(peer = %peer_id, "Connection closed");
                    self.deregister_connection(&peer_id, &connection_id);

                    // If this was the last connection with the peer, emit a `PeerDisconnected` event.
                    if self.peer_connections_count(&peer_id) == 0 {
                        return Some(ServiceOut::PeerDisconnected(peer_id));
                    }
                }
                SwarmEvent::AddressChange {
                    connection_id, new, ..
                } => {
                    let new_remote_addr = new.get_remote_address();
                    self.update_connection_remote_address(&connection_id, new_remote_addr.clone());
                }
                // TODO: Add support for connection (dial and listen) errors
                _ => {}
            },
        }

        None
    }
}
