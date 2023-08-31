use std::net::Ipv4Addr;

use assert_matches::assert_matches;
use libp2p::identity::PeerId;
use libp2p::swarm::ConnectionId;
use libp2p::Multiaddr;
use rand::Rng;

use common_test as testlib;
use common_test::service::noop_context;

use super::{ConnectionsInEvent, ConnectionsOutEvent, ConnectionsService, ConnectionsSwarmEvent};

/// Convenience function to create a new `ConnectionId` for testing.
fn new_test_connection_id() -> ConnectionId {
    let id = rand::thread_rng().gen::<usize>();
    ConnectionId::new_unchecked(id)
}

/// Convenience function to create a new `PeerId` for testing.
fn new_test_peer_id() -> PeerId {
    PeerId::random()
}

/// Convenience function to create a new `Multiaddr` for testing.
///
/// The `Multiaddr` will contain a random IPv4 address and a random TCP port. The peer ID is not
/// included.
fn new_test_multiaddr() -> Multiaddr {
    let ip_addr = rand::thread_rng().gen::<[u8; 4]>();
    let tcp_port = rand::thread_rng().gen::<u16>();
    Multiaddr::from_iter([
        libp2p::core::multiaddr::Protocol::Ip4(Ipv4Addr::from(ip_addr)),
        libp2p::core::multiaddr::Protocol::Tcp(tcp_port),
    ])
}

/// Creates a sequence of events that simulate an inbound connection establishment.
fn new_inbound_connection_seq(
    connection_id: ConnectionId,
    peer_id: PeerId,
    local_addr: Multiaddr,
    remote_addr: Multiaddr,
) -> impl IntoIterator<Item = ConnectionsInEvent> {
    [
        ConnectionsInEvent::EstablishedInboundConnection {
            connection_id,
            peer_id,
            local_addr,
            remote_addr,
        },
        ConnectionsInEvent::SwarmEvent(ConnectionsSwarmEvent::ConnectionEstablished {
            connection_id,
            peer_id,
        }),
    ]
}

/// Create a sequence of events that simulate an outbound connection establishment.
fn new_outbound_connection_seq(
    connection_id: ConnectionId,
    peer_id: PeerId,
    remote_addr: Multiaddr,
) -> impl IntoIterator<Item = ConnectionsInEvent> {
    [
        ConnectionsInEvent::EstablishedOutboundConnection {
            connection_id,
            peer_id,
            remote_addr,
        },
        ConnectionsInEvent::SwarmEvent(ConnectionsSwarmEvent::ConnectionEstablished {
            connection_id,
            peer_id,
        }),
    ]
}

/// Create a sequence of events that simulate a connection close.
fn new_connection_closed_seq(
    connection_id: ConnectionId,
    peer_id: PeerId,
) -> impl IntoIterator<Item = ConnectionsInEvent> {
    [ConnectionsInEvent::SwarmEvent(
        ConnectionsSwarmEvent::ConnectionClosed {
            connection_id,
            peer_id,
        },
    )]
}

#[test]
fn new_inbound_connection_established() {
    //// Given
    let mut service = testlib::service::default_test_service::<ConnectionsService>();

    let local_addr = new_test_multiaddr();
    let remote_addr = new_test_multiaddr();
    let remote_peer_id = new_test_peer_id();

    //// When
    // Simulate an inbound connection established
    let input_events = new_inbound_connection_seq(
        new_test_connection_id(),
        remote_peer_id,
        local_addr.clone(),
        remote_addr.clone(),
    );

    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert_eq!(
        service.active_peers_count(),
        1,
        "Only one peer should be active"
    );
    assert!(
        service.active_peers().contains(&remote_peer_id),
        "Peer should be active"
    );
    assert_eq!(
        service.peer_connections_count(&remote_peer_id),
        1,
        "Peer should have 1 active connection"
    );
}

#[test]
fn new_outbound_connection_established() {
    //// Given
    let mut service = testlib::service::default_test_service::<ConnectionsService>();

    let remote_addr = new_test_multiaddr();
    let remote_peer_id = new_test_peer_id();

    //// When
    // Simulate an outbound connection established
    let input_events = new_outbound_connection_seq(
        new_test_connection_id(),
        remote_peer_id,
        remote_addr.clone(),
    );

    testlib::service::inject_events(&mut service, input_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert_eq!(
        service.active_peers_count(),
        1,
        "Only one peer should be active"
    );
    assert!(
        service.active_peers().contains(&remote_peer_id),
        "Peer should be active"
    );
    assert_eq!(
        service.peer_connections_count(&remote_peer_id),
        1,
        "Peer should have 1 active connection"
    );
}

#[test]
fn emit_new_peer_connected_event_on_first_inbound_connection() {
    //// Given
    let mut service = testlib::service::default_test_service::<ConnectionsService>();

    let local_addr = new_test_multiaddr();
    let remote_addr = new_test_multiaddr();
    let remote_peer_id = new_test_peer_id();

    //// When
    // Simulate three inbound connections established from the same peer
    let input_events = itertools::chain!(
        new_inbound_connection_seq(
            new_test_connection_id(),
            remote_peer_id,
            local_addr.clone(),
            remote_addr.clone(),
        ),
        new_inbound_connection_seq(
            new_test_connection_id(),
            remote_peer_id,
            local_addr.clone(),
            remote_addr.clone(),
        ),
        new_inbound_connection_seq(
            new_test_connection_id(),
            remote_peer_id,
            local_addr.clone(),
            remote_addr.clone(),
        ),
    );

    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert_eq!(
        service.active_peers_count(),
        1,
        "Only one peer should be active"
    );
    assert!(
        service.active_peers().contains(&remote_peer_id),
        "Peer should be active"
    );
    assert_eq!(
        service.peer_connections_count(&remote_peer_id),
        3,
        "Peer should have 3 active connections"
    );

    // Assert output events
    assert_eq!(output_events.len(), 1, "Only one event should be emitted");
    assert_matches!(output_events[0], ConnectionsOutEvent::NewPeerConnected(peer_id) => {
            assert_eq!(peer_id, peer_id);
    }, "A NewPeerConnected event for peer should be emitted");
}

#[test]
fn emit_new_peer_connected_event_on_first_outbound_connection() {
    //// Given
    let mut service = testlib::service::default_test_service::<ConnectionsService>();

    let remote_addr = new_test_multiaddr();
    let remote_peer_id = new_test_peer_id();

    //// When
    // Simulate three outbound connections established to the same peer
    let input_events = itertools::chain!(
        new_outbound_connection_seq(
            new_test_connection_id(),
            remote_peer_id,
            remote_addr.clone()
        ),
        new_outbound_connection_seq(
            new_test_connection_id(),
            remote_peer_id,
            remote_addr.clone()
        ),
        new_outbound_connection_seq(
            new_test_connection_id(),
            remote_peer_id,
            remote_addr.clone()
        ),
    );

    testlib::service::inject_events(&mut service, input_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert_eq!(
        service.active_peers_count(),
        1,
        "Only one peer should be active"
    );
    assert!(
        service.active_peers().contains(&remote_peer_id),
        "Peer should be active"
    );
    assert_eq!(
        service.peer_connections_count(&remote_peer_id),
        3,
        "Peer should have 3 active connections"
    );

    // Assert output events
    assert_eq!(output_events.len(), 1, "Only one event should be emitted");
    assert_matches!(output_events[0], ConnectionsOutEvent::NewPeerConnected(peer_id) => {
            assert_eq!(peer_id, peer_id);
    }, "A NewPeerConnected event for peer should be emitted");
}

#[test]
fn should_not_peer_disconnected_event_if_remaining_connections() {
    //// Given
    let mut service = testlib::service::default_test_service::<ConnectionsService>();

    let inbound_connection_id = new_test_connection_id();
    let outbound_connection_id = new_test_connection_id();

    let local_addr = new_test_multiaddr();
    let remote_addr = new_test_multiaddr();
    let remote_peer_id = new_test_peer_id();

    // Simulate two connections established to the same peer
    let conn_established_events = itertools::chain!(
        new_outbound_connection_seq(outbound_connection_id, remote_peer_id, remote_addr.clone()),
        new_inbound_connection_seq(
            inbound_connection_id,
            remote_peer_id,
            local_addr.clone(),
            remote_addr.clone(),
        ),
    );
    testlib::service::inject_events(&mut service, conn_established_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the inbound connections close
    let conn_closed_events = new_connection_closed_seq(inbound_connection_id, remote_peer_id);
    testlib::service::inject_events(&mut service, conn_closed_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert_eq!(
        service.active_peers_count(),
        1,
        "Only one peer should be active"
    );
    assert!(
        service.active_peers().contains(&remote_peer_id),
        "Peer should be active"
    );
    assert_eq!(
        service.peer_connections_count(&remote_peer_id),
        1,
        "Peer should have 1 active connection"
    );

    // Assert output events
    assert_eq!(output_events.len(), 0, "No events should be emitted");
}

#[test]
fn emit_peer_disconnected_event_when_no_remaining_connections() {
    //// Given
    let mut service = testlib::service::default_test_service::<ConnectionsService>();

    let inbound_connection_id = new_test_connection_id();
    let outbound_connection_id = new_test_connection_id();

    let local_addr = new_test_multiaddr();
    let remote_addr = new_test_multiaddr();
    let remote_peer_id = new_test_peer_id();

    // Simulate two connections established to the same peer
    let conn_established_events = itertools::chain!(
        new_outbound_connection_seq(outbound_connection_id, remote_peer_id, remote_addr.clone()),
        new_inbound_connection_seq(
            inbound_connection_id,
            remote_peer_id,
            local_addr.clone(),
            remote_addr.clone(),
        ),
    );
    testlib::service::inject_events(&mut service, conn_established_events);
    testlib::service::poll(&mut service, &mut noop_context());

    //// When
    // Simulate the connections close
    let conn_closed_events = itertools::chain!(
        new_connection_closed_seq(inbound_connection_id, remote_peer_id),
        new_connection_closed_seq(outbound_connection_id, remote_peer_id),
    );
    testlib::service::inject_events(&mut service, conn_closed_events);

    let output_events = testlib::service::collect_events(&mut service, &mut noop_context());

    //// Then
    // Assert state
    assert!(
        service.active_peers().is_empty(),
        "No peers should be active"
    );
    assert_eq!(
        service.peer_connections_count(&remote_peer_id),
        0,
        "Peer should not have any active connections"
    );

    // Assert output events
    assert_eq!(output_events.len(), 1, "Only one event should be emitted");
    assert_matches!(output_events[0], ConnectionsOutEvent::PeerDisconnected(peer_id) => {
            assert_eq!(peer_id, peer_id);
    }, "A PeerDisconnected event for peer should be emitted");
}

#[test]
#[ignore]
fn handle_connection_address_change() {
    todo!()
}
