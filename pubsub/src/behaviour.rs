use std::collections::{BTreeSet, VecDeque};
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

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
    validate_frame_proto, validate_message_proto, validate_subopts_proto, Frame, FrameProto,
    Message as FrameMessage, SubscriptionAction,
};
use crate::message::Message;
use crate::protocol::{
    Protocol, ProtocolRouterConnectionEvent, ProtocolRouterInEvent, ProtocolRouterOutEvent,
    ProtocolRouterSubscriptionEvent,
};
use crate::services::connections::{
    ConnectionsInEvent, ConnectionsOutEvent, ConnectionsService, ConnectionsSwarmEvent,
};
use crate::services::message_cache::{
    MessageCacheInEvent, MessageCacheService, MessageCacheSubscriptionEvent,
};
use crate::services::subscriptions::{
    SubscriptionsInEvent, SubscriptionsOutEvent, SubscriptionsPeerConnectionEvent,
    SubscriptionsService,
};
use crate::subscription::Subscription;
use crate::topic::{Hasher, Topic, TopicHash};

pub struct Behaviour<P: Protocol> {
    /// The behaviour's configuration.
    config: Config,

    /// Peer connections tracking and management service.
    connections_service: ServiceContext<ConnectionsService>,

    /// Peer subscriptions tracking and management service.
    subscriptions_service: ServiceContext<SubscriptionsService>,

    /// Message cache and deduplication service.
    message_cache_service: ServiceContext<MessageCacheService>,

    /// The pubsub protocol router service.
    protocol_router_service: ServiceContext<P::RouterService>,

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
impl<P: Protocol> Behaviour<P> {
    /// Creates a new `Behaviour` from the given configuration and protocol.
    pub fn new(config: Config, protocol: P) -> Self {
        let message_cache_service = ServiceContext::new(MessageCacheService::new(
            config.message_cache_capacity(),
            config.message_cache_ttl(),
            config.heartbeat_interval(),
            Duration::from_secs(0),
        ));
        let protocol_router_service = ServiceContext::new(protocol.router());

        Self {
            config,
            connections_service: Default::default(),
            subscriptions_service: Default::default(),
            message_cache_service,
            protocol_router_service,
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
        let topic = message.topic();

        tracing::debug!(%topic, "Publishing message");

        // Check if we are subscribed to the topic.
        if !self.subscriptions_service.is_subscribed(&topic) {
            return Err(anyhow::anyhow!("Not subscribed to topic"));
        }

        // Check if we have connections to publish the message.
        if self.connections_service.active_peers_count() == 0 {
            return Err(anyhow::anyhow!("No active connections"));
        }

        // Check if message was already published.
        if self.message_cache_service.contains(&message) {
            return Err(anyhow::anyhow!("Message already published"));
        }

        let message = Rc::new(message);

        // Notify the message cache service to add the message to the message cache.
        self.message_cache_service
            .do_send(MessageCacheInEvent::MessagePublished(message.clone()));

        // Emit a message published event to the protocol service.
        self.protocol_router_service
            .do_send(ProtocolRouterInEvent::MessagePublished(message));

        Ok(())
    }
}

/// Internal API.
impl<P: Protocol> Behaviour<P> {
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
            let messages = frame
                .publish
                .into_iter()
                .filter_map(|msg| {
                    if let Err(err) = validate_message_proto(&msg) {
                        tracing::trace!(%src, "Received invalid message: {}", err);
                        return None;
                    }

                    Some(FrameMessage::from(msg))
                })
                // Filter out messages from topics that we are not subscribed to.
                .filter(|msg| self.subscriptions_service.is_subscribed(&msg.topic()))
                // Filter out messages that we have already received.
                .filter(|msg| !self.message_cache_service.contains(msg))
                .map(Rc::new)
                .collect::<Vec<_>>();

            tracing::trace!(%src, messages=messages.len(), "Received messages");

            for message in messages {
                // Add the message to the message cache.
                self.message_cache_service
                    .do_send(MessageCacheInEvent::MessageReceived(message.clone()));

                // Emit a message received event to the protocol's routing service.
                self.protocol_router_service
                    .do_send(ProtocolRouterInEvent::MessageReceived {
                        src,
                        message: message.clone(),
                    });

                // Emit a message received event to the behaviour output mailbox.
                self.behaviour_output_mailbox
                    .push_back(ToSwarm::GenerateEvent(Event::MessageReceived {
                        src,
                        message: (*message).clone(), // Clone the underlying message.
                    }));
            }
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

impl<P: Protocol> NetworkBehaviour for Behaviour<P> {
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
            P::protocol_id(),
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
            P::protocol_id(),
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
                .do_send(SubscriptionsInEvent::from_peer_connection_event(
                    conn_event.clone(),
                ));

            // Notify the protocol's routing service of the connection event.
            self.protocol_router_service.do_send(match conn_event {
                ConnectionsOutEvent::NewPeerConnected(peer) => {
                    ProtocolRouterInEvent::ConnectionEvent(
                        ProtocolRouterConnectionEvent::PeerConnected(peer),
                    )
                }
                ConnectionsOutEvent::PeerDisconnected(peer) => {
                    ProtocolRouterInEvent::ConnectionEvent(
                        ProtocolRouterConnectionEvent::PeerDisconnected(peer),
                    )
                }
            });
        }

        // Poll the subscriptions service.
        while let Poll::Ready(sub_event) = self.subscriptions_service.poll(cx) {
            match sub_event {
                SubscriptionsOutEvent::Subscribed(sub) => {
                    // Notify the message cache service of the subscription.
                    self.message_cache_service
                        .do_send(MessageCacheInEvent::SubscriptionEvent(
                            MessageCacheSubscriptionEvent::Subscribed {
                                topic: sub.topic.clone(),
                                message_id_fn: sub.message_id_fn.clone(),
                            },
                        ));

                    // Notify the protocol's routing service of the subscription event.
                    self.protocol_router_service
                        .do_send(ProtocolRouterInEvent::SubscriptionEvent(
                            ProtocolRouterSubscriptionEvent::Subscribed(sub.clone()),
                        ));

                    // Send the subscription update to all active peers.
                    tracing::debug!(topic = %sub.topic, "Sending subscription update");

                    let frame =
                        Frame::new_with_subscriptions([SubscriptionAction::Subscribe(sub.topic)]);
                    for dst_peer in self.connections_service.active_peers() {
                        self.send_frame(dst_peer, frame.clone());
                    }
                }
                SubscriptionsOutEvent::Unsubscribed(topic) => {
                    // Notify the message cache service of the unsubscription.
                    self.message_cache_service
                        .do_send(MessageCacheInEvent::SubscriptionEvent(
                            MessageCacheSubscriptionEvent::Unsubscribed(topic.clone()),
                        ));

                    // Notify the protocol's service of the unsubscription event.
                    self.protocol_router_service
                        .do_send(ProtocolRouterInEvent::SubscriptionEvent(
                            ProtocolRouterSubscriptionEvent::Unsubscribed(topic.clone()),
                        ));

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

                    // Notify the protocol's service of the peer subscription event.
                    self.protocol_router_service
                        .do_send(ProtocolRouterInEvent::SubscriptionEvent(
                            ProtocolRouterSubscriptionEvent::PeerSubscribed { peer, topic },
                        ));
                }
                SubscriptionsOutEvent::PeerUnsubscribed { peer, topic } => {
                    tracing::debug!(src = %peer, %topic, "Peer unsubscribed");

                    // Notify the protocol's service of the peer unsubscription event.
                    self.protocol_router_service
                        .do_send(ProtocolRouterInEvent::SubscriptionEvent(
                            ProtocolRouterSubscriptionEvent::PeerUnsubscribed { peer, topic },
                        ));
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

        // Poll the message cache service.
        let _ = self.message_cache_service.poll(cx);

        // Poll the protocol service.
        while let Poll::Ready(event) = self.protocol_router_service.poll(cx) {
            match event {
                ProtocolRouterOutEvent::ForwardMessage { message, dest } => {
                    for dst_peer in dest {
                        let message = message.clone();

                        let frame = Frame::new_with_messages([
                            // TODO: Avoid cloning the message here.
                            (*message).clone(), // Clone the underlying message.
                        ]);
                        self.send_frame(dst_peer, frame);
                    }
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
