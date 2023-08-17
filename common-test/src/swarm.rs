#![allow(dead_code)]

use std::fmt::Debug;
use std::time::Duration;

use assert_matches::assert_matches;
use futures::StreamExt;
use libp2p::core::transport::ListenerId;
use libp2p::swarm::{ConnectionHandler, NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, Swarm};

type NetworkBehaviourEvent<B> = <B as NetworkBehaviour>::ToSwarm;
type NetworkBehaviourConnectionHandler<B> = <B as NetworkBehaviour>::ConnectionHandler;
type NetworkBehaviourConnectionHandlerError<B> =
    <NetworkBehaviourConnectionHandler<B> as ConnectionHandler>::Error;

/// Poll the swarm to make the async runtime progress.
pub async fn poll<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(swarm: &mut Swarm<B>) {
    let swarm_peer_id = *swarm.local_peer_id();
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("PeerId: {} > Event emitted: {:?}", swarm_peer_id, event);
    }
}

/// Poll the different swarms for a given period of time.
pub async fn poll_mesh<
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
>(
    duration: Duration,
    swarm1: &mut Swarm<B1>,
    swarm2: &mut Swarm<B2>,
) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(duration) => break,
            _ = poll(swarm1) => {},
            _ = poll(swarm2) => {},
        }
    }
}

/// Poll the mesh's swarms for events for a given duration and collect them.
pub async fn poll_mesh_and_collect_events<B1: NetworkBehaviour, B2: NetworkBehaviour>(
    duration: Duration,
    swarm1: &mut Swarm<B1>,
    swarm2: &mut Swarm<B2>,
) -> (
    Vec<SwarmEvent<NetworkBehaviourEvent<B1>, NetworkBehaviourConnectionHandlerError<B1>>>,
    Vec<SwarmEvent<NetworkBehaviourEvent<B2>, NetworkBehaviourConnectionHandlerError<B2>>>,
) {
    let mut swarm1_events = Vec::new();
    let mut swarm2_events = Vec::new();

    loop {
        tokio::select! {
            _ = tokio::time::sleep(duration) => break,
            event = swarm1.select_next_some() => swarm1_events.push(event),
            event = swarm2.select_next_some() => swarm2_events.push(event),
        }
    }

    (swarm1_events, swarm2_events)
}

/// Listen on the given address and assert that the listen is successful.
pub fn should_listen_on_address<B: NetworkBehaviour>(
    swarm: &mut Swarm<B>,
    addr: Multiaddr,
) -> ListenerId {
    let result = swarm.listen_on(addr);

    assert_matches!(result, Ok(_), "listen on address should succeed");

    result.unwrap()
}

/// Wait the swarm for the [`SwarmEvent::NewListenAddr`] event and return the new listen address.
pub async fn wait_for_new_listen_addr<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(
    swarm: &mut Swarm<B>,
) -> Multiaddr {
    let swarm_peer_id = *swarm.local_peer_id();
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("PeerId: {} > Event emitted: {:?}", swarm_peer_id, event);
        if let SwarmEvent::NewListenAddr { address, .. } = event {
            return address;
        }
    }
}

/// Dial the given address and assert that the dial is successful.
pub fn should_dial_address<B: NetworkBehaviour>(swarm: &mut Swarm<B>, addr: Multiaddr) {
    let result = swarm.dial(addr);

    assert_matches!(result, Ok(_), "dial address should succeed");
}

/// Wait the swarm for the [`SwarmEvent::IncomingConnection`] event.
pub async fn wait_for_incoming_connection<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(
    swarm: &mut Swarm<B>,
) {
    let swarm_peer_id = *swarm.local_peer_id();
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("PeerId: {} > Event emitted: {:?}", swarm_peer_id, event);
        if matches!(event, SwarmEvent::IncomingConnection { .. }) {
            break;
        }
    }
}

/// Wait the swarm for the [`SwarmEvent::Dialing`] event.
pub async fn wait_for_dialing<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(swarm: &mut Swarm<B>) {
    let swarm_peer_id = *swarm.local_peer_id();
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("PeerId: {} > Event emitted: {:?}", swarm_peer_id, event);
        if matches!(event, SwarmEvent::Dialing { .. }) {
            break;
        }
    }
}

/// Wait the swarm for the [`SwarmEvent::ConnectionEstablished`] event.
pub async fn wait_for_connection_established<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(
    swarm: &mut Swarm<B>,
) {
    let swarm_peer_id = *swarm.local_peer_id();
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("PeerId: {} > Event emitted: {:?}", swarm_peer_id, event);
        if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
            break;
        }
    }
}

/// Wait for all swarms' [`SwarmEvent:NewListenAddress`] event to be emitted and return the new
/// listen addresses.
pub async fn wait_for_start_listening<
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
>(
    publisher: &mut Swarm<B1>,
    subscriber: &mut Swarm<B2>,
) -> (Multiaddr, Multiaddr) {
    tokio::join!(
        wait_for_new_listen_addr(publisher),
        wait_for_new_listen_addr(subscriber)
    )
}

/// Wait for the connection established event to be emitted by the different swarms.
pub async fn wait_for_connection_establishment<
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
>(
    dialer: &mut Swarm<B1>,
    receiver: &mut Swarm<B2>,
) {
    tokio::join!(
        wait_for_connection_established(dialer),
        wait_for_connection_established(receiver)
    );
}
