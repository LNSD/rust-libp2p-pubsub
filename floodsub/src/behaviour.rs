use std::collections::{HashMap, VecDeque};
use std::task::{Context, Poll};

use libp2p::core::Endpoint;
use libp2p::identity::PeerId;
use libp2p::swarm::behaviour::ConnectionEstablished;
use libp2p::swarm::{
    AddressChange, ConnectionClosed, ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour,
    NotifyHandler, PollParameters, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::Multiaddr;

use crate::config::Config;
use crate::connections::ConnectionManager;
use crate::frame::{Frame, Message, SubscriptionAction};
use crate::handler::{Command as HandlerCommand, Event as HandlerEvent, Handler};
use crate::proto::{
    fragment_rpc_message, validate_message_proto, validate_rpc_proto, validate_subopts_proto,
    FragmentationError, RpcProto,
};
use crate::router::Router;
use crate::seqno::{LinearSequenceNumber, MessageSeqNumberGenerator};
use crate::topic::{Hasher, Topic, TopicHash};

pub const FLOODSUB_PROTOCOL_ID: &str = "/floodsub/1.0.0";

/// Events that can be produced by the behaviour.
#[derive(Debug)]
pub enum Event {
    /// Message received.
    Message {
        /// Peer that propagated the message.
        source: PeerId,

        /// Message topic.
        topic: TopicHash,

        /// Message.
        message: Message,
    },
}

/// Errors that can happen when sending a RPC frame to a peer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    /// Insufficient peers to send message to.
    ///
    /// This error is returned when there are not enough peers subscribed to the topic to
    /// propagate the message.
    #[error("insufficient peers")]
    InsufficientPeers,

    /// Failed to fragment the message.
    #[error("failed to fragment message")]
    FragmentationFailed(#[from] FragmentationError),
}

/// Errors that can happen when publishing a message.
#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PublishError {
    /// Not subscribed to the topic.
    #[error("not subscribed to topic")]
    NotSubscribed,

    /// Frame sending failed.
    ///
    /// This error is returned when the frame could not be sent to the peer.
    #[error("failed to send frame: {0}")]
    MessagePublishFailed(#[from] SendError),
}

/// Errors that can happen when subscribing/unsubscribing to a topic.
#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionError {
    /// Failed to publish subscriptions.
    #[error("failed to publish subscription: {0}")]
    SubscriptionPublishFailed(#[from] SendError),
}

pub struct Behaviour {
    /// Protocol behaviour configuration.
    config: Config,

    /// Events that need to be yielded to the swarm when polling.
    swarm_out_events: VecDeque<ToSwarm<Event, HandlerCommand>>,

    /// Connection manager.
    ///
    /// This is used to keep track of the connections and their state, and to manage the
    /// connections associated with certain peer.
    connections: ConnectionManager,

    /// Floodsub router.
    ///
    /// This is used to keep track of the subscriptions and to route the messages to the
    /// appropriate subscribers.
    router: Router,

    /// Message sequence number generator.
    message_seqno_generator: Box<dyn MessageSeqNumberGenerator + Send>,

    /// Message author.
    message_author: Option<PeerId>,
}

/// Public API.
impl Behaviour {
    /// Create a new behaviour instance.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            swarm_out_events: Default::default(),
            connections: Default::default(),
            router: Default::default(),
            message_seqno_generator: Box::new(LinearSequenceNumber::new()),
            message_author: None,
        }
    }

    /// Get a reference to the connection manager.
    pub fn connections(&self) -> &ConnectionManager {
        &self.connections
    }

    /// Get a reference to the router.
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Subscribe to topic.
    ///
    /// Returns `Ok(true)` if the subscription was successful, `Ok(false)` if we were already
    /// subscribed to the topic.
    ///
    /// Subscribing to a topic publishes immediately the subscription to the network. This requires
    /// at least one active connection to the network, otherwise the subscription will fail with a
    /// [`SubscriptionError::SubscriptionPublishFailed`].
    pub fn subscribe<H: Hasher>(&mut self, topic: &Topic<H>) -> Result<bool, SubscriptionError> {
        log::debug!("Subscribing to topic {topic}");

        let topic = topic.hash();

        if self.router.is_subscribed(&topic) {
            return Ok(false);
        }

        // TODO: Add a subscription filter.

        // Add the subscription to the router.
        self.router.subscribe(topic.clone());

        // If there are no active connections, as we cannot publish the subscription, return.
        if self.connections.active_peers_count() == 0 {
            return Ok(true);
        }

        // Publish the subscription to the network.
        let subscription = SubscriptionAction::subscribe(topic.clone());
        let frame = Frame::new_with_subscriptions(vec![subscription]);

        for peer in self.connections.active_peers() {
            if let Err(err) = self.send_rpc_frame(&peer, frame.clone()) {
                log::debug!("Failed to send topic {topic} subscription to peer {peer}: {err}");
            }
        }

        Ok(true)
    }

    /// Unsubscribe from topic.
    ///
    /// Returns `Ok(true)` if the unsubscription was successful, `Ok(false)` if we were not
    /// subscribed to the topic.
    pub fn unsubscribe<H: Hasher>(&mut self, topic: &Topic<H>) -> Result<bool, SubscriptionError> {
        log::debug!("Unsubscribing from topic {topic}");

        let topic = topic.hash();

        if !self.router.is_subscribed(&topic) {
            return Ok(false);
        }

        // TODO: Add a subscription filter.

        // Remove the subscription from the router.
        self.router.unsubscribe(&topic);

        // If there are no active connections, as we cannot publish the subscription, return.
        if self.connections.active_peers_count() == 0 {
            return Ok(true);
        }

        // Publish the subscription to the network.
        let subscription = SubscriptionAction::unsubscribe(topic.clone());
        let frame = Frame::new_with_subscriptions(vec![subscription]);

        for peer in self.connections.active_peers() {
            if let Err(err) = self.send_rpc_frame(&peer, frame.clone()) {
                log::debug!("Failed to send topic {topic} unsubscription to peer {peer}: {err}");
            }
        }

        Ok(true)
    }

    /// Publish a message to the network.
    pub fn publish<H: Hasher>(
        &mut self,
        topic: &Topic<H>,
        data: impl Into<Vec<u8>>,
    ) -> Result<(), PublishError> {
        log::debug!("Publishing message to topic {topic}");

        let topic = topic.hash();

        // Check if we are subscribed to the topic.
        if !self.router.is_subscribed(&topic) {
            return Err(PublishError::NotSubscribed);
        }

        let propagation_peers: Vec<PeerId> =
            self.router.propagation_routes(&topic).into_iter().collect();

        // Check if we have enough connections to publish the message.
        if propagation_peers.is_empty() {
            return Err(PublishError::MessagePublishFailed(
                SendError::InsufficientPeers,
            ));
        }

        // Build the message.
        let data = data.into();
        let author = self.message_author;
        let seqno = self.message_seqno_generator.next();

        let message = {
            let mut msg = Message::new(topic, data);
            msg.set_source(author);
            msg.set_sequence_number(seqno);
            msg
        };

        // TODO: Sing the message.

        let frame = Frame::new_with_messages(vec![message]);
        for peer in propagation_peers {
            if let Err(err) = self.send_rpc_frame(&peer, frame.clone()) {
                log::debug!("Failed to send message to peer {peer}: {err}");
            }
        }

        Ok(())
    }
}

/// Connection handling.
impl Behaviour {
    fn on_connection_established(&mut self, event: ConnectionEstablished) {
        // Event's `other_established` is the number of connections existing before the connection
        // was established.
        self.connections
            .on_connection_established(&event.peer_id, &event.connection_id);

        let connections_count = self.connections.peer_connections_count(&event.peer_id);
        debug_assert_eq!(
            connections_count,
            event.other_established + 1,
            "Peer connections count should match the other established connections plus one"
        );

        // If this is the first connection with the peer, send our subscriptions to the peer.
        // The peer will be added to the router when the subscriptions are received.
        if connections_count == 1 {
            log::debug!("Connection established with {}", event.peer_id);

            let subscriptions = self
                .router
                .subscriptions()
                .cloned()
                .map(SubscriptionAction::subscribe);
            let frame = Frame::new_with_subscriptions(subscriptions);

            if let Err(err) = self.send_rpc_frame(&event.peer_id, frame) {
                log::warn!("Failed to send subscriptions to {}: {}", event.peer_id, err);
            }
        }
    }

    fn on_connection_closed(
        &mut self,
        event: ConnectionClosed<<Self as NetworkBehaviour>::ConnectionHandler>,
    ) {
        // Event's `remaining_established` is the number of connections remaining after the closed
        // connection is removed.
        self.connections
            .on_connection_closed(&event.peer_id, &event.connection_id);

        let peer_connections = self.connections.peer_connections_count(&event.peer_id);
        debug_assert_eq!(
            peer_connections, event.remaining_established,
            "Peer connections count should match the remaining established connections"
        );

        // If there are no more connections with the peer, remove the peer from the router.
        if peer_connections == 0 {
            log::debug!("No connections remaining for peer {}", event.peer_id);

            self.router.remove_peer(&event.peer_id);
        }
    }

    fn on_connection_address_change(&mut self, event: AddressChange) {
        let new_remote_address = event.new.get_remote_address();
        self.connections
            .on_address_change(&event.connection_id, new_remote_address.clone());
    }
}

/// Event emission (e.g., connection handler, swarm, application).
impl Behaviour {
    /// Emit a behaviour event to the application.
    ///
    /// This function will queue the event to be emitted to the application. It will be emitted
    /// when the swarm is polled.
    fn emit_behaviour_event(&mut self, event: Event) {
        self.swarm_out_events
            .push_back(ToSwarm::GenerateEvent(event));
    }

    /// Emit a event to the connection handlers.
    ///
    /// This function will queue the event to be emitted to the connection handlers. It will be
    /// emitted when the behaviour is polled.
    fn emit_handler_event(&mut self, peer: &PeerId, event: HandlerCommand, handler: NotifyHandler) {
        self.swarm_out_events.push_back(ToSwarm::NotifyHandler {
            peer_id: *peer,
            event,
            handler,
        });
    }
}

/// RPC frames handling.
impl Behaviour {
    /// Handle received RPC frame.
    ///
    /// This function is called when a peer sends us an RPC frame. The frame is validated and
    /// converted to the appropriate messages and subscriptions.
    fn on_received_rpc_frame(&mut self, src: &PeerId, frame: RpcProto) {
        // First: Validate the RPC frame.
        if let Err(err) = validate_rpc_proto(&frame) {
            log::trace!("Received invalid RPC frame from {}: {}", src, err);
            return;
        }

        // Second: Validate, sanitize and convert protobuf into messages.
        let messages = frame.publish.into_iter().filter_map(|msg| {
            if let Err(err) = validate_message_proto(&msg) {
                log::trace!("Received invalid message from {}: {}", src, err);
                return None;
            }

            Some(Into::<Message>::into(msg))
        });

        self.handle_received_messages(src, messages);

        // Third: Validate, sanitize and convert protobuf  into subscription actions.
        let subscriptions = frame.subscriptions.into_iter().filter_map(|sub| {
            if let Err(err) = validate_subopts_proto(&sub) {
                log::trace!("Received invalid subscription from {}: {}", src, err);
                return None;
            }

            Some(Into::<SubscriptionAction>::into(sub))
        });

        self.handle_received_subscriptions(src, subscriptions);
    }

    /// Handle received messages.
    ///
    /// This function is called when a peer sends us a message. The messages are filtered and
    /// forwarded to the appropriate subscribers.
    fn handle_received_messages(&mut self, src: &PeerId, messages: impl Iterator<Item = Message>) {
        // Filter out messages from topics that we are not subscribed to.
        let messages = messages.filter(|msg| self.router.is_subscribed(&msg.topic()));

        // Filter out messages that we have already seen.
        // TODO: Filter seen messages (e.g., implement a seen cache)

        // Validate the messages.
        // TODO: Add message validation logic (e.g., check the message signature).

        let messages = messages.collect::<Vec<_>>();

        // If there are no messages to forward, return.
        if messages.is_empty() {
            return;
        }

        // Emit the messages to the application.
        for msg in messages.iter() {
            log::trace!("Received message from {src} to topic {}", msg.topic_str());
            self.emit_behaviour_event(Event::Message {
                source: *src,
                topic: msg.topic(),
                message: msg.clone(),
            })
        }

        // Forward the messages to the appropriate subscribers. Group the messages that are
        // destined to the same topic.
        let peer_frames = messages
            .into_iter()
            .fold(HashMap::<PeerId, Vec<Message>>::new(), |mut mmap, msg| {
                let next_hops = self
                    .router
                    .propagation_routes(&msg.topic())
                    .into_iter()
                    .filter(|peer| {
                        // Don't send the message back to the propagation source.
                        peer != src
                    });

                for peer in next_hops {
                    mmap.entry(peer).or_default().push(msg.clone());
                }

                mmap
            })
            .into_iter()
            .map(|(peer, messages)| (peer, Frame::new_with_messages(messages)));

        for (peer, frame) in peer_frames {
            if let Err(err) = self.send_rpc_frame(&peer, frame) {
                log::debug!("Failed to send RPC frame to {}: {}", peer, err);
            }
        }
    }

    /// Handle received subscriptions.
    ///
    /// This function will add or remove the peer topic subscriptions from the router.
    fn handle_received_subscriptions(
        &mut self,
        src: &PeerId,
        subscriptions: impl Iterator<Item = SubscriptionAction>,
    ) {
        for sub in subscriptions {
            match sub {
                SubscriptionAction::Subscribe(topic) => {
                    self.router.add_peer_subscription(*src, topic);
                }
                SubscriptionAction::Unsubscribe(topic) => {
                    self.router.remove_peer_subscription(src, &topic);
                }
            }
        }
    }

    /// Send an RPC frame to a peer.
    ///
    /// This function will fragment the RPC frame into multiple frames if it exceeds the maximum
    /// frame size.
    fn send_rpc_frame(
        &mut self,
        dst: &PeerId,
        frame: impl Into<RpcProto>,
    ) -> Result<(), SendError> {
        let frames = fragment_rpc_message(frame.into(), self.config.max_frame_size)?;

        // Send the RPC frame(s) to any active connection with the peer
        for frame in frames {
            self.emit_handler_event(dst, HandlerCommand::SendFrame(frame), NotifyHandler::Any);
        }

        Ok(())
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
        self.connections.register_inbound(
            connection_id,
            peer,
            local_addr.clone(),
            remote_addr.clone(),
        );

        Ok(Handler::new(
            FLOODSUB_PROTOCOL_ID,
            self.config.max_frame_size,
            self.config.connection_idle_timeout,
        ))
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        remote_addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.connections
            .register_outbound(connection_id, peer, remote_addr.clone());

        Ok(Handler::new(
            FLOODSUB_PROTOCOL_ID,
            self.config.max_frame_size,
            self.config.connection_idle_timeout,
        ))
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        match event {
            FromSwarm::ConnectionEstablished(ev) => self.on_connection_established(ev),
            FromSwarm::ConnectionClosed(ev) => self.on_connection_closed(ev),
            FromSwarm::AddressChange(ev) => self.on_connection_address_change(ev),
            _ => {}
        }
    }

    fn on_connection_handler_event(
        &mut self,
        src: PeerId,
        connection: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        match event {
            HandlerEvent::FrameReceived(frame) => self.on_received_rpc_frame(&src, frame),
            HandlerEvent::Disabled(reason) => {
                log::debug!("Connection handler {connection:?} for peer {src} disabled: {reason}");
            }
        }
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Some(event) = self.swarm_out_events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
