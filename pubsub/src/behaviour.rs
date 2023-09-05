use std::collections::{BTreeSet, VecDeque};
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use libp2p::core::Endpoint;
use libp2p::identity::PeerId;
use libp2p::swarm::behaviour::ConnectionEstablished;
use libp2p::swarm::{
    AddressChange, ConnectionClosed, ConnectionDenied, ConnectionHandler, ConnectionId,
    DialFailure, FromSwarm, ListenFailure, NetworkBehaviour, NotifyHandler, PollParameters,
    THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::Multiaddr;

use common::service::{BufferedContext, ServiceContext};

use crate::config::Config;
use crate::conn_handler::{Command as HandlerCommand, Event as HandlerEvent, Handler};
use crate::event::Event;
use crate::framing::{Message as FrameMessage, SubscriptionAction};
use crate::message::Message;
use crate::protocol::{
    Protocol, ProtocolRouterConnectionEvent, ProtocolRouterInEvent, ProtocolRouterOutEvent,
    ProtocolRouterSubscriptionEvent,
};
use crate::services::connections::{
    ConnectionsInEvent, ConnectionsOutEvent, ConnectionsService, ConnectionsSwarmEvent,
};
use crate::services::framing::{
    FramingDownstreamInEvent, FramingDownstreamOutEvent, FramingInEvent, FramingOutEvent,
    FramingServiceContext, FramingUpstreamInEvent, FramingUpstreamOutEvent,
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
use crate::upgrade::SimpleProtocolUpgrade;

pub struct Behaviour<P: Protocol> {
    /// The behaviour's configuration.
    config: Config,

    /// Peer connections tracking and management service.
    connections_service: BufferedContext<ConnectionsService>,

    /// Peer subscriptions tracking and management service.
    subscriptions_service: BufferedContext<SubscriptionsService>,

    /// Message cache and deduplication service.
    message_cache_service: BufferedContext<MessageCacheService>,

    /// The pubsub protocol router service.
    protocol_router_service: BufferedContext<P::RouterService>,

    /// The frame encoder and decoder service.
    framing_service: FramingServiceContext,

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
        let message_cache_service = BufferedContext::new(MessageCacheService::new(
            config.message_cache_capacity(),
            config.message_cache_ttl(),
            config.heartbeat_interval(),
            Duration::from_secs(0),
        ));
        let protocol_router_service = BufferedContext::new(protocol.router());

        Self {
            config,
            connections_service: Default::default(),
            subscriptions_service: Default::default(),
            message_cache_service,
            protocol_router_service,
            framing_service: Default::default(),
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
        let topic = message.topic.clone();

        tracing::debug!(%topic, "Publishing message");

        // Check if we are subscribed to the topic.
        if !self.subscriptions_service.is_subscribed(&topic) {
            return Err(anyhow::anyhow!("Not subscribed to topic"));
        }

        // Check if we have connections to publish the message.
        if self.connections_service.active_peers_count() == 0 {
            return Err(anyhow::anyhow!("No active connections"));
        }

        let message = FrameMessage::from(message);

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
    /// Send a pubsub frame to a `dst` peer.
    ///
    /// This method checks if the frame size is within the allowed limits and queues a connection
    /// handler event to send the frame to the peer.
    fn send_frame(&mut self, dest: PeerId, frame: Bytes) {
        tracing::trace!(%dest, "Sending frame");

        // Check if the frame size exceeds the maximum allowed size. If so, drop the frame.
        if frame.len() > self.config.max_frame_size() {
            tracing::warn!(%dest, "Frame size exceeds maximum allowed size");
            return;
        }

        self.conn_handler_mailbox.push_back(ToSwarm::NotifyHandler {
            peer_id: dest,
            handler: NotifyHandler::Any,
            event: HandlerCommand::SendFrame(frame),
        });
    }
}

impl<P: Protocol> NetworkBehaviour for Behaviour<P> {
    type ConnectionHandler = Handler<SimpleProtocolUpgrade<&'static str>>;
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
            SimpleProtocolUpgrade::new(P::protocol_id()),
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
            SimpleProtocolUpgrade::new(P::protocol_id()),
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
            HandlerEvent::FrameReceived(frame) => {
                // Notify the framing service of the received frame handler event.
                self.framing_service.do_send(FramingInEvent::Upstream(
                    FramingUpstreamInEvent::RawFrameReceived {
                        src: peer_id,
                        frame,
                    },
                ));
            }
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

                    let sub_action = SubscriptionAction::Subscribe(sub.topic);
                    for dest in self.connections_service.active_peers() {
                        // Notify the framing service of the subscription update request.
                        self.framing_service.do_send(FramingInEvent::Downstream(
                            FramingDownstreamInEvent::SendSubscriptionRequest {
                                dest,
                                actions: vec![sub_action.clone()],
                            },
                        ));
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

                    let sub_action = SubscriptionAction::Unsubscribe(topic);
                    for dest in self.connections_service.active_peers() {
                        // Notify the framing service of the subscription update request.
                        self.framing_service.do_send(FramingInEvent::Downstream(
                            FramingDownstreamInEvent::SendSubscriptionRequest {
                                dest,
                                actions: vec![sub_action.clone()],
                            },
                        ));
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
                SubscriptionsOutEvent::SendSubscriptions { dest, topics } => {
                    // Send the subscriptions to the peer.
                    tracing::debug!(%dest, ?topics, "Sending subscriptions");

                    let actions = topics
                        .into_iter()
                        .map(SubscriptionAction::Subscribe)
                        .collect::<Vec<_>>();
                    self.framing_service.do_send(FramingInEvent::Downstream(
                        FramingDownstreamInEvent::SendSubscriptionRequest { dest, actions },
                    ));
                }
            }
        }

        // Poll the message cache service.
        let _ = self.message_cache_service.poll(cx);

        // Poll the protocol service.
        while let Poll::Ready(event) = self.protocol_router_service.poll(cx) {
            match event {
                ProtocolRouterOutEvent::ForwardMessage { message, dest } => {
                    for dest in dest {
                        // Notify the framing service of the message to send.
                        self.framing_service.do_send(FramingInEvent::Downstream(
                            FramingDownstreamInEvent::ForwardMessage {
                                dest,
                                message: message.clone(),
                            },
                        ));
                    }
                }
            }
        }

        // Poll the framing service.
        while let Poll::Ready(event) = self.framing_service.poll(cx) {
            match event {
                FramingOutEvent::Downstream(FramingDownstreamOutEvent::SendFrame {
                    dest,
                    frame,
                }) => {
                    // Send the frame to the peer.
                    self.send_frame(dest, frame);
                }
                FramingOutEvent::Upstream(ev) => match ev {
                    FramingUpstreamOutEvent::MessageReceived { src, message } => {
                        // Skip the message if we are not subscribed to the topic or the message
                        // was already received.
                        if !self.subscriptions_service.is_subscribed(&message.topic())
                            || self.message_cache_service.contains(&message)
                        {
                            continue;
                        }

                        // Notify the message cache service of the received message.
                        self.message_cache_service
                            .do_send(MessageCacheInEvent::MessageReceived(message.clone()));

                        // Notify the behaviour output mailbox of the received message.
                        self.behaviour_output_mailbox
                            .push_back(ToSwarm::GenerateEvent(Event::MessageReceived {
                                src,
                                message: (*message).clone().into(),
                            }));

                        // Notify the protocol's routing service of the received message.
                        self.protocol_router_service
                            .do_send(ProtocolRouterInEvent::MessageReceived { src, message });
                    }
                    FramingUpstreamOutEvent::SubscriptionRequestReceived { src, action } => {
                        match &action {
                            SubscriptionAction::Subscribe(topic)
                                if !self.subscriptions_service.is_peer_subscribed(&src, topic) =>
                            {
                                // Notify the subscriptions service of the subscription request.
                                self.subscriptions_service.do_send(
                                    SubscriptionsInEvent::PeerSubscriptionRequest { src, action },
                                );
                            }
                            SubscriptionAction::Unsubscribe(topic)
                                if self.subscriptions_service.is_peer_subscribed(&src, topic) =>
                            {
                                // Notify the subscriptions service of the unsubscription request.
                                self.subscriptions_service.do_send(
                                    SubscriptionsInEvent::PeerSubscriptionRequest { src, action },
                                );
                            }
                            _ => {}
                        }
                    }
                },
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

impl From<Message> for FrameMessage {
    fn from(message: Message) -> Self {
        let mut msg = Self::new(message.topic, message.data);
        msg.set_sequence_number(message.sequence_number);
        msg.set_key(message.key);
        msg.set_source(message.from);
        msg.set_signature(message.signature);
        msg
    }
}

impl From<FrameMessage> for Message {
    fn from(message: FrameMessage) -> Self {
        Self {
            topic: message.topic(),
            data: message.data().to_vec(),
            sequence_number: message.sequence_number(),
            key: message.key().map(ToOwned::to_owned),
            from: message.source(),
            signature: message.signature().map(ToOwned::to_owned),
        }
    }
}
