use std::collections::{BTreeSet, VecDeque};
use std::task::{Context, Poll};

use libp2p::core::Endpoint;
use libp2p::identity::PeerId;
use libp2p::swarm::behaviour::ConnectionEstablished;
use libp2p::swarm::{
    AddressChange, ConnectionClosed, ConnectionDenied, ConnectionHandler, ConnectionId,
    DialFailure, FromSwarm, ListenFailure, NetworkBehaviour, NotifyHandler, PollParameters,
    THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::Multiaddr;
use prost::Message as _;

use common::service::Context as ServiceContext;

use crate::config::Config;
use crate::conn_handler::{Command as HandlerCommand, Event as HandlerEvent, Handler};
use crate::event::Event;
use crate::framing::{
    validate_frame_proto, validate_subopts_proto, Frame, FrameProto, Message, SubscriptionAction,
};
use crate::services::connections::{
    ConnectionsInEvent, ConnectionsOutEvent, ConnectionsService, ConnectionsSwarmEvent,
};
use crate::services::subscriptions::{
    SubscriptionsInEvent, SubscriptionsOutEvent, SubscriptionsPeerConnectionEvent,
    SubscriptionsService,
};
use crate::subscription::Subscription;
use crate::topic::{Hasher, Topic, TopicHash};

// TODO: Make the connection handler generic over the protocol.
const PROTOCOL_ID: &str = "/floodsub/1.0.0";

pub struct Behaviour {
    /// The behaviour's configuration.
    config: Config,

    /// Peer connections tracking and management service.
    connections_service: ServiceContext<ConnectionsService>,

    /// Peer subscriptions tracking and management service.
    subscriptions_service: ServiceContext<SubscriptionsService>,

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
            subscriptions_service: Default::default(),
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
        self.subscriptions_service.subscriptions()
    }

    /// Get peer topic subscriptions.
    pub fn peer_subscriptions(&self, peer_id: &PeerId) -> Option<&BTreeSet<TopicHash>> {
        self.subscriptions_service.peer_subscriptions(peer_id)
    }

    /// Subscribe to topic.
    ///
    /// Returns `Ok(true)` if the subscription was successful, `Ok(false)` if we were already
    /// subscribed to the topic.
    pub fn subscribe(&mut self, sub: impl Into<Subscription>) -> anyhow::Result<bool> {
        let sub = sub.into();

        tracing::debug!(?sub, "Subscribing to topic");

        if self.subscriptions_service.is_subscribed(&sub.topic) {
            return Ok(false);
        }

        // Notify the subscriptions service of the subscription request.
        self.subscriptions_service
            .do_send(SubscriptionsInEvent::SubscriptionRequest(sub));

        Ok(true)
    }

    /// Unsubscribe from topic.
    ///
    /// Returns `Ok(true)` if the unsubscription was successful, `Ok(false)` if we were not
    /// subscribed to the topic.
    pub fn unsubscribe<H: Hasher>(&mut self, topic: &Topic<H>) -> anyhow::Result<bool> {
        tracing::debug!(sub = %topic, "Unsubscribing from topic");

        let topic = topic.hash();

        if !self.subscriptions_service.is_subscribed(&topic) {
            return Ok(false);
        }

        // Notify the subscriptions service of the unsubscription request.
        self.subscriptions_service
            .do_send(SubscriptionsInEvent::UnsubscriptionRequest(topic));

        Ok(true)
    }

    /// Publish a message to the network.
    pub fn publish(&mut self, message: Message) -> anyhow::Result<()> {
        todo!()
    }
}

/// Internal API.
impl Behaviour {
    /// Handle a pubsub frame received from a `src` peer.
    fn on_frame_received(&mut self, src: PeerId, frame: FrameProto) {
        // 1. Validate the RPC frame.
        if let Err(err) = validate_frame_proto(&frame) {
            tracing::trace!(%src, "Received invalid frame: {}", err);
            return;
        }

        tracing::trace!(%src, "Frame received");

        // 2. Validate, sanitize and process the frame messages'.
        if !frame.publish.is_empty() {
            // TODO: Implement the frame messages processing.
        }

        // 3. Validate, sanitize and process the frame subscription actions.
        if !frame.subscriptions.is_empty() {
            let sub_actions = frame.subscriptions.into_iter().filter_map(|sub| {
                if let Err(err) = validate_subopts_proto(&sub) {
                    tracing::trace!(%src, "Received invalid subscription action: {}", err);
                    return None;
                }

                let sub = SubscriptionAction::from(sub);

                // Filter out duplicated subscription actions.
                match &sub {
                    SubscriptionAction::Subscribe(topic) => {
                        if self.subscriptions_service.is_peer_subscribed(&src, topic).unwrap_or(false) {
                            tracing::trace!(%src, topic = %topic, "Received duplicated subscription action");
                            return None;
                        }
                    }
                    SubscriptionAction::Unsubscribe(topic) => {
                        if !self.subscriptions_service.is_peer_subscribed(&src, topic).unwrap_or(false) {
                            tracing::trace!(%src, topic = %topic, "Received unsubscription action for non-subscribed topic");
                            return None;
                        }
                    }
                }

                Some(sub)
            })
                .collect::<Vec<_>>();

            for action in sub_actions {
                self.subscriptions_service
                    .do_send(SubscriptionsInEvent::PeerSubscriptionRequest { src, action });
            }
        }

        // 4. Validate, sanitize and process the frame control messages.
        // TODO: Implement the frame control messages processing.
    }

    /// Send a pubsub frame to a `dst` peer.
    ///
    /// This method checks if the frame size is within the allowed limits and queues a connection
    /// handler event to send the frame to the peer.
    fn send_frame(&mut self, dst: PeerId, frame: impl Into<FrameProto>) {
        tracing::trace!(dst = %dst, "Sending frame");

        let frame_proto = frame.into();

        // Check if the frame size exceeds the maximum allowed size. If so, drop the frame.
        // TODO: Implement frame fragmentation.
        if frame_proto.encoded_len() > self.config.max_frame_size() {
            tracing::warn!(dst = %dst, "Frame size exceeds maximum allowed size");
            return;
        }

        self.conn_handler_mailbox.push_back(ToSwarm::NotifyHandler {
            peer_id: dst,
            handler: NotifyHandler::Any,
            event: HandlerCommand::SendFrame(frame_proto),
        });
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
        _connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        match event {
            HandlerEvent::FrameReceived(ev) => self.on_frame_received(peer_id, ev),
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        // Poll the connections service.
        while let Poll::Ready(conn_event) = self.connections_service.poll(cx) {
            // Notify the subscriptions service of the connection event.
            self.subscriptions_service
                .do_send(SubscriptionsInEvent::from_peer_connection_event(conn_event));

            // TODO: Notify the protocol's routing service of the connection event.
        }

        // Poll the subscriptions service.
        while let Poll::Ready(sub_event) = self.subscriptions_service.poll(cx) {
            match sub_event {
                SubscriptionsOutEvent::Subscribed(sub) => {
                    // TODO: Notify the message cache service of the subscription.

                    // TODO: Notify the protocol service of the subscription.

                    // Send the subscription update to all active peers.
                    tracing::debug!(topic = %sub.topic, "Sending subscription update");

                    let frame =
                        Frame::new_with_subscriptions([SubscriptionAction::Subscribe(sub.topic)]);
                    for dst_peer in self.connections_service.active_peers() {
                        self.send_frame(dst_peer, frame.clone());
                    }
                }
                SubscriptionsOutEvent::Unsubscribed(topic) => {
                    // TODO: Notify the message cache service of the unsubscription.

                    // TODO: Notify the protocol service of the unsubscription.

                    // Send the subscription updates to all active peers.
                    tracing::debug!(%topic, "Sending subscription update");

                    let frame =
                        Frame::new_with_subscriptions([SubscriptionAction::Unsubscribe(topic)]);
                    for dst_peer in self.connections_service.active_peers() {
                        self.send_frame(dst_peer, frame.clone());
                    }
                }
                SubscriptionsOutEvent::PeerSubscribed { peer, topic } => {
                    tracing::debug!(src = %peer, %topic, "Peer subscribed");

                    // TODO: Notify the protocol service of the peer subscription.
                }
                SubscriptionsOutEvent::PeerUnsubscribed { peer, topic } => {
                    tracing::debug!(src = %peer, %topic, "Peer unsubscribed");

                    // TODO: Notify the protocol service of the peer unsubscription.
                }
                SubscriptionsOutEvent::SendSubscriptions { peer, topics } => {
                    // Send the subscriptions to the peer.
                    tracing::debug!(dst = %peer, ?topics, "Sending subscriptions");

                    let frame = Frame::new_with_subscriptions(
                        topics.into_iter().map(SubscriptionAction::Subscribe),
                    );
                    self.send_frame(peer, frame);
                }
            }
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

impl From<ConnectionsOutEvent> for SubscriptionsPeerConnectionEvent {
    fn from(ev: ConnectionsOutEvent) -> Self {
        match ev {
            ConnectionsOutEvent::NewPeerConnected(peer) => {
                SubscriptionsPeerConnectionEvent::NewPeerConnected(peer)
            }
            ConnectionsOutEvent::PeerDisconnected(peer) => {
                SubscriptionsPeerConnectionEvent::PeerDisconnected(peer)
            }
        }
    }
}
