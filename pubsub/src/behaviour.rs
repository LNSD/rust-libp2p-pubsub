use std::collections::{BTreeSet, VecDeque};
use std::task::{Context, Poll};

use libp2p::core::Endpoint;
use libp2p::identity::PeerId;
use libp2p::swarm::{
    ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour, PollParameters, THandler,
    THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::Multiaddr;

use crate::config::Config;
use crate::conn_handler::{Command as HandlerCommand, Handler};
use crate::event::Event;
use crate::framing::Message;
use crate::services::connections::ConnectionsService;
use crate::subscription::Subscription;
use crate::topic::{Hasher, Topic, TopicHash};

// TODO: Make the connection handler generic over the protocol.
const PROTOCOL_ID: &str = "/floodsub/1.0.0";

pub struct Behaviour {
    /// The behaviour's configuration.
    config: Config,

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
            conn_handler_mailbox: Default::default(),
            behaviour_output_mailbox: Default::default(),
        }
    }

    /// Get a reference to the connections service.
    pub fn connections(&self) -> &ConnectionsService {
        todo!()
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
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        // TODO: Handle inbound connection event

        Ok(Handler::new(
            PROTOCOL_ID,
            self.config.max_frame_size(),
            self.config.connection_idle_timeout(),
        ))
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        // TODO: Handle outbound connection event

        Ok(Handler::new(
            PROTOCOL_ID,
            self.config.max_frame_size(),
            self.config.connection_idle_timeout(),
        ))
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        // TODO: Handle connection events
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
