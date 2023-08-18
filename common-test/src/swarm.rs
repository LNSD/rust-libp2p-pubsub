use std::fmt::Debug;
use std::time::Duration;

use assert_matches::assert_matches;
use futures::StreamExt;
use libp2p::core::transport::ListenerId;
use libp2p::swarm::{ConnectionHandler, NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, Swarm};
use tracing_futures::Instrument;

type NetworkBehaviourEvent<B> = <B as NetworkBehaviour>::ToSwarm;
type NetworkBehaviourConnectionHandler<B> = <B as NetworkBehaviour>::ConnectionHandler;
type NetworkBehaviourConnectionHandlerError<B> =
    <NetworkBehaviourConnectionHandler<B> as ConnectionHandler>::Error;
type NetworkBehaviourSwarmEvent<B> =
    SwarmEvent<NetworkBehaviourEvent<B>, NetworkBehaviourConnectionHandlerError<B>>;

/// Poll the swarm to make the async runtime progress.
#[tracing::instrument(fields(swarm = % swarm.local_peer_id()))]
pub async fn poll<B, E>(swarm: &mut Swarm<B>)
where
    B: NetworkBehaviour<ToSwarm = E>,
    E: Debug,
{
    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(event = ?event);
    }
}

/// Poll the different swarms for a given period of time.
#[tracing::instrument(skip_all)]
pub async fn poll_mesh<B1, E1, B2, E2>(
    duration: Duration,
    swarm1: &mut Swarm<B1>,
    swarm2: &mut Swarm<B2>,
) where
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
{
    loop {
        tokio::select! {
            _ = tokio::time::sleep(duration) => break,
            _ = poll(swarm1) => {},
            _ = poll(swarm2) => {},
        }
    }
}

/// Poll the mesh's swarms for events for a given duration and collect them.
#[tracing::instrument(skip_all)]
pub async fn poll_mesh_and_collect_events<B1, E1, B2, E2>(
    duration: Duration,
    swarm1: &mut Swarm<B1>,
    swarm2: &mut Swarm<B2>,
) -> (
    Vec<NetworkBehaviourSwarmEvent<B1>>,
    Vec<NetworkBehaviourSwarmEvent<B2>>,
)
where
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
{
    let swarm1_span =
        tracing::trace_span!("poll_mesh_and_collect_events", swarm = %swarm1.local_peer_id());
    let swarm2_span =
        tracing::trace_span!("poll_mesh_and_collect_events", swarm = %swarm2.local_peer_id());

    let mut swarm1_events = Vec::new();
    let mut swarm2_events = Vec::new();

    loop {
        tokio::select! {
            _ = tokio::time::sleep(duration) => break,
            event = swarm1.select_next_some().instrument(swarm1_span.clone()) => {
                tracing::trace!(parent: &swarm1_span, event = ?event);
                swarm1_events.push(event);
            },
            event = swarm2.select_next_some().instrument(swarm2_span.clone()) => {
                tracing::trace!(parent: &swarm2_span, event = ?event);
                swarm2_events.push(event);
            },
        }
    }

    (swarm1_events, swarm2_events)
}

/// Listen on the given address and assert that the listen is successful.
#[tracing::instrument(skip_all, fields(swarm = % swarm.local_peer_id()))]
pub fn should_listen_on_address<B>(swarm: &mut Swarm<B>, addr: Multiaddr) -> ListenerId
where
    B: NetworkBehaviour,
{
    tracing::debug!("listen on address: {addr}");

    let result = swarm.listen_on(addr);

    assert_matches!(result, Ok(_), "listen on address should succeed");

    result.unwrap()
}

/// Wait the swarm for the [`SwarmEvent::NewListenAddr`] event and return the new listen address.
#[tracing::instrument(fields(swarm = % swarm.local_peer_id()))]
pub async fn wait_for_new_listen_addr<B, E>(swarm: &mut Swarm<B>) -> Multiaddr
where
    B: NetworkBehaviour<ToSwarm = E>,
    E: Debug,
{
    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(event = ?event);
        if let SwarmEvent::NewListenAddr { address, .. } = event {
            return address;
        }
    }
}

/// Dial the given address and assert that the dial is successful.
#[tracing::instrument(skip_all, fields(swarm = % swarm.local_peer_id()))]
pub fn should_dial_address<B>(swarm: &mut Swarm<B>, addr: Multiaddr)
where
    B: NetworkBehaviour,
{
    tracing::debug!("dialing to address: {addr}");

    let result = swarm.dial(addr);

    assert_matches!(result, Ok(_), "dial address should succeed");
}

/// Wait the swarm for the [`SwarmEvent::IncomingConnection`] event.
#[tracing::instrument(fields(swarm = % swarm.local_peer_id()))]
pub async fn wait_for_incoming_connection<B, E>(swarm: &mut Swarm<B>)
where
    B: NetworkBehaviour<ToSwarm = E>,
    E: Debug,
{
    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(event = ?event);
        if matches!(event, SwarmEvent::IncomingConnection { .. }) {
            break;
        }
    }
}

/// Wait the swarm for the [`SwarmEvent::Dialing`] event.
#[tracing::instrument(fields(swarm = % swarm.local_peer_id()))]
pub async fn wait_for_dialing<B, E>(swarm: &mut Swarm<B>)
where
    B: NetworkBehaviour<ToSwarm = E>,
    E: Debug,
{
    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(event = ?event);
        if matches!(event, SwarmEvent::Dialing { .. }) {
            break;
        }
    }
}

/// Wait the swarm for the [`SwarmEvent::ConnectionEstablished`] event.
#[tracing::instrument(fields(swarm = % swarm.local_peer_id()))]
pub async fn wait_for_connection_established<B, E>(swarm: &mut Swarm<B>)
where
    B: NetworkBehaviour<ToSwarm = E>,
    E: Debug,
{
    loop {
        let event = swarm.select_next_some().await;
        tracing::trace!(event = ?event);
        if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
            break;
        }
    }
}

/// Wait for all swarms' [`SwarmEvent:NewListenAddress`] event to be emitted and return the new
/// listen addresses.
#[tracing::instrument(skip_all)]
pub async fn wait_for_start_listening<B1, E1, B2, E2>(
    swarm1: &mut Swarm<B1>,
    swarm2: &mut Swarm<B2>,
) -> (Multiaddr, Multiaddr)
where
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
{
    tokio::join!(
        wait_for_new_listen_addr(swarm1),
        wait_for_new_listen_addr(swarm2)
    )
}

/// Wait for the connection established event to be emitted by the different swarms.
#[tracing::instrument(skip_all)]
pub async fn wait_for_connection_establishment<B1, E1, B2, E2>(
    dialer: &mut Swarm<B1>,
    receiver: &mut Swarm<B2>,
) where
    B1: NetworkBehaviour<ToSwarm = E1>,
    E1: Debug,
    B2: NetworkBehaviour<ToSwarm = E2>,
    E2: Debug,
{
    tokio::join!(
        wait_for_connection_established(dialer),
        wait_for_connection_established(receiver)
    );
}
