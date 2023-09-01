use std::collections::{BTreeSet, VecDeque};
use std::task::{Context, Poll};

use libp2p::core::Endpoint;
use libp2p::identity::PeerId;
use libp2p::swarm::behaviour::ConnectionEstablished;
use libp2p::swarm::{
    AddressChange, ConnectionClosed, ConnectionDenied, ConnectionHandler, ConnectionId,
    DialFailure, FromSwarm, ListenFailure, NetworkBehaviour, PollParameters, THandler,
    THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::Multiaddr;

use common::service::Context as ServiceContext;

use crate::config::Config;
use crate::conn_handler::{Command as HandlerCommand, Handler};
use crate::event::Event;
use crate::framing::Message;
use crate::services::connections::{ConnectionsInEvent, ConnectionsService, ConnectionsSwarmEvent};
use crate::subscription::Subscription;
use crate::topic::{Hasher, Topic, TopicHash};

// TODO: Make the connection handler generic over the protocol.
const PROTOCOL_ID: &str = "/floodsub/1.0.0";

pub struct Behaviour {
    /// The behaviour's configuration.
    config: Config,

    /// Peer connections tracking and management service.
    connections_service: ServiceContext<ConnectionsService>,

    /// Connection handler's mailbox.
    ///
    /// It should only contain [`ToSwarm::NotifyHandler`] events to send to the connection handler.
    conn_handler_mailbox: VecDeque<ToSwarm<Event, HandlerCommand>>,

    /// Behaviour output events mailbox.
    ///
    /// It should only contain [`ToSwarm::GenerateEvent`] events to send out of the behaviour, to
    /// the application.
    behaviour_output_mailbox: VecDeque<ToSwarm<Event, HandlerCommand>>,
}

/// Public API.
impl Behaviour {
    /// Creates a new `Behaviour` from the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            connections_service: Default::default(),
            conn_handler_mailbox: Default::default(),
            behaviour_output_mailbox: Default::default(),
        }
    }

    /// Get a reference to the connections service.
    pub fn connections(&self) -> &ConnectionsService {
        &self.connections_service
    }

    /// Get local node topic subscriptions.
    pub fn subscriptions(&self) -> &BTreeSet<TopicHash> {
        todo!()
    }

    /// Get peer topic subscriptions.
    pub fn peer_subscriptions(&self, peer_id: &PeerId) -> Option<&BTreeSet<TopicHash>> {
        todo!()
    }

    /// Subscribe to topic.
    ///
    /// Returns `Ok(true)` if the subscription was successful, `Ok(false)` if we were already
    /// subscribed to the topic.
    ///
    /// Subscribing to a topic publishes immediately the subscription to the network. This requires
    /// at least one active connection to the network, otherwise the subscription will fail with a
    /// [`SubscriptionError::SubscriptionPublishFailed`].
    pub fn subscribe(&mut self, sub: impl Into<Subscription>) -> anyhow::Result<bool> {
        todo!()
    }

    /// Unsubscribe from topic.
    ///
    /// Returns `Ok(true)` if the unsubscription was successful, `Ok(false)` if we were not
    /// subscribed to the topic.
    pub fn unsubscribe<H: Hasher>(&mut self, topic: &Topic<H>) -> anyhow::Result<bool> {
        todo!()
    }

    /// Publish a message to the network.
    pub fn publish(&mut self, message: Message) -> anyhow::Result<()> {
        todo!()
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = Handler;
    type ToSwarm = Event;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer_id: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        // Emit an event to the connections service.
        self.connections_service
            .do_send(ConnectionsInEvent::EstablishedInboundConnection {
                connection_id,
                peer_id,
                local_addr: local_addr.clone(),
                remote_addr: remote_addr.clone(),
            });

        Ok(Handler::new(
            PROTOCOL_ID,
            self.config.max_frame_size(),
            self.config.connection_idle_timeout(),
        ))
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer_id: PeerId,
        remote_addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        // Emit an event to the connections service.
        self.connections_service
            .do_send(ConnectionsInEvent::EstablishedOutboundConnection {
                connection_id,
                peer_id,
                remote_addr: remote_addr.clone(),
            });

        Ok(Handler::new(
            PROTOCOL_ID,
            self.config.max_frame_size(),
            self.config.connection_idle_timeout(),
        ))
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        match event {
            FromSwarm::ConnectionEstablished(ev) => {
                self.connections_service
                    .do_send(ConnectionsInEvent::from_swarm_event(ev));
            }
            FromSwarm::ConnectionClosed(ev) => {
                self.connections_service
                    .do_send(ConnectionsInEvent::from_swarm_event(ev));
            }
            FromSwarm::AddressChange(ev) => {
                self.connections_service
                    .do_send(ConnectionsInEvent::from_swarm_event(ev));
            }
            FromSwarm::DialFailure(ev) => {
                self.connections_service
                    .do_send(ConnectionsInEvent::from_swarm_event(ev));
            }
            FromSwarm::ListenFailure(ev) => {
                self.connections_service
                    .do_send(ConnectionsInEvent::from_swarm_event(ev));
            }
            _ => {}
        }
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        // TODO: Frame received event
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        // Poll the connections service.
        while let Poll::Ready(_event) = self.connections_service.poll(cx) {
            // TODO: Notify the subscriptions service of the connection event.

            // TODO: Notify the protocol's routing service of the connection event.
        }

        // Process the connection handler mailbox.
        if let Some(event) = self.conn_handler_mailbox.pop_front() {
            return Poll::Ready(event);
        }

        // Process the behaviour output events mailbox.
        if let Some(event) = self.behaviour_output_mailbox.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}

impl From<ConnectionEstablished<'_>> for ConnectionsSwarmEvent {
    fn from(ev: ConnectionEstablished) -> Self {
        Self::ConnectionEstablished {
            connection_id: ev.connection_id,
            peer_id: ev.peer_id,
        }
    }
}

impl<H: ConnectionHandler> From<ConnectionClosed<'_, H>> for ConnectionsSwarmEvent {
    fn from(ev: ConnectionClosed<H>) -> Self {
        Self::ConnectionClosed {
            connection_id: ev.connection_id,
            peer_id: ev.peer_id,
        }
    }
}

impl From<AddressChange<'_>> for ConnectionsSwarmEvent {
    fn from(ev: AddressChange) -> Self {
        Self::AddressChange {
            connection_id: ev.connection_id,
            peer_id: ev.peer_id,
            old: ev.old.clone(),
            new: ev.new.clone(),
        }
    }
}

impl From<DialFailure<'_>> for ConnectionsSwarmEvent {
    fn from(ev: DialFailure) -> Self {
        Self::DialFailure {
            connection_id: ev.connection_id,
            peer_id: ev.peer_id,
            error: ev.error.to_string(), // TODO: Use a custom error type.
        }
    }
}

impl From<ListenFailure<'_>> for ConnectionsSwarmEvent {
    fn from(ev: ListenFailure) -> Self {
        Self::ListenFailure {
            connection_id: ev.connection_id,
            local_addr: ev.local_addr.clone(),
            send_back_addr: ev.send_back_addr.clone(),
            error: ev.error.to_string(), // TODO: Use a custom error type.
        }
    }
}
