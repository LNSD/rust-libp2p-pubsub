#![allow(dead_code)]

use std::fmt::Debug;

use futures::StreamExt;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, Swarm};

pub async fn poll<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(swarm: &mut Swarm<B>) {
    loop {
        let event = swarm.select_next_some().await;
        log::error!("Event: {:?}", event);
    }
}

pub async fn wait_for_new_listen_addr<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(
    swarm: &mut Swarm<B>,
) -> Multiaddr {
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("Event: {:?}", event);
        if let SwarmEvent::NewListenAddr { address, .. } = event {
            return address;
        }
    }
}

pub async fn wait_for_incoming_connection<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(
    swarm: &mut Swarm<B>,
) {
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("Event: {:?}", event);
        if matches!(event, SwarmEvent::IncomingConnection { .. }) {
            break;
        }
    }
}

pub async fn wait_for_dialing<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(swarm: &mut Swarm<B>) {
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("Event: {:?}", event);
        if matches!(event, SwarmEvent::Dialing { .. }) {
            break;
        }
    }
}

pub async fn wait_for_connection_established<B: NetworkBehaviour<ToSwarm = E>, E: Debug>(
    swarm: &mut Swarm<B>,
) {
    loop {
        let event = swarm.select_next_some().await;
        log::trace!("Event: {:?}", event);
        if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
            break;
        }
    }
}
