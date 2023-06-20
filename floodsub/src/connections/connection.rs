use libp2p::Multiaddr;

/// The direction of a connection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConnectionDirection {
    /// The connection is inbound.
    Inbound,

    /// The connection is outbound.
    Outbound,
}

/// The state of a connection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// The connection has been established, but substreams negotiation is in progress.
    ///
    /// This is the initial state of a connection.
    /// It is also the state of a connection after a disconnection.
    /// In this state, the connection is not ready to be used.
    /// It will be moved to `Established` once the connection is established.
    #[default]
    Connecting,

    /// The connection is established and ready to be used.
    ///
    /// This is the state of a connection once it is established and the substreams negotiated.
    /// In this state, the connection is ready to be used.
    Established,
}

/// A connection.
#[derive(Debug)]
pub struct Connection {
    /// The direction of the connection.
    direction: ConnectionDirection,

    /// The connection state.
    state: ConnectionState,

    /// The connection local address.
    ///
    /// This is `None` if the connection is outbound.
    local_addr: Option<Multiaddr>,

    /// The connection remote address.
    remote_addr: Multiaddr,
}

impl Connection {
    /// Creates a new [`ConnectionDirection::Inbound`] connection instance.
    ///
    /// The connection is in the [`ConnectionState::Connecting`] state.
    pub(crate) fn new_inbound(local_addr: Multiaddr, remote_addr: Multiaddr) -> Self {
        Self {
            local_addr: Some(local_addr),
            remote_addr,
            state: ConnectionState::Connecting,
            direction: ConnectionDirection::Inbound,
        }
    }

    /// Creates a new [`ConnectionDirection::Outbound`] connection instance.
    ///
    /// The connection is in the [`ConnectionState::Connecting`] state.
    pub(crate) fn new_outbound(remote_addr: Multiaddr) -> Self {
        Self {
            local_addr: None,
            remote_addr,
            state: ConnectionState::Connecting,
            direction: ConnectionDirection::Outbound,
        }
    }

    /// Update connection state.
    pub(crate) fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }

    /// Update connection remote address.
    pub(crate) fn set_remote_address(&mut self, remote_addr: Multiaddr) {
        self.remote_addr = remote_addr;
    }

    /// Whether the connection is inbound.
    #[must_use]
    pub fn is_inbound(&self) -> bool {
        self.direction == ConnectionDirection::Inbound
    }

    /// Whether the connection is outbound.
    #[must_use]
    pub fn is_outbound(&self) -> bool {
        self.direction == ConnectionDirection::Outbound
    }

    /// Whether the connection is in established state.
    #[must_use]
    pub fn is_established(&self) -> bool {
        self.state == ConnectionState::Established
    }
}
