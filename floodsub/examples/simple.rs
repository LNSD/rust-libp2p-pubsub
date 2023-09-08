use std::process::exit;

use futures::StreamExt;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::core::upgrade;
use libp2p::identity::Keypair;
use libp2p::plaintext::PlainText2Config;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{dns, tcp, yamux, Multiaddr, PeerId, Swarm, Transport};

use libp2p_pubsub_core::{Behaviour, Config, IdentTopic, Message};
use libp2p_pubsub_floodsub::Protocol as Floodsub;

/// Set up a DNS-enabled TCP transport over the Yamux protocol.
fn new_dns_tcp_transport(keypair: &Keypair) -> Boxed<(PeerId, StreamMuxerBox)> {
    let transport = dns::TokioDnsConfig::system(tcp::tokio::Transport::new(
        tcp::Config::default().nodelay(true),
    ))
    .expect("Failed to create DNS/-enabled TCP transport");

    transport
        .upgrade(upgrade::Version::V1)
        .authenticate(PlainText2Config {
            local_public_key: keypair.public(),
        })
        .multiplex(yamux::Config::default())
        .timeout(std::time::Duration::from_secs(20))
        .boxed()
}

/// Create a new Floodsub node with the given keypair.
fn new_floodsub_node(keypair: &Keypair) -> Swarm<Behaviour<Floodsub>> {
    let peer_id = PeerId::from(keypair.public());
    let transport = new_dns_tcp_transport(keypair);
    let behaviour = Behaviour::new(Config::default(), Floodsub);
    SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build()
}

/// The topic to publish/subscribe to.
const PUBSUB_TOPIC: &str = "/examples/simple-topic";

/// The addresses to listen on and dial to for the publisher and subscriber.
const PUBLISHER_LISTEN_MULTIADDDR: &str = "/ip4/0.0.0.0/tcp/60100";
const PUBLISHER_DIAL_MULTIADDDR: &str = "/dns4/localhost/tcp/60100";
const SUBSCRIBER_LISTEN_MULTIADDDR: &str = "/ip4/0.0.0.0/tcp/60101";
const SUBSCRIBER_DIAL_MULTIADDDR: &str = "/dns4/localhost/tcp/60101";

/// A simple example where two nodes, one publisher and one subscriber, are subscribed to the same
/// topic. The publisher sends a message and the subscriber receives it.
#[tokio::main]
async fn main() {
    // Create a message to publish with the topic.
    let pubsub_topic = IdentTopic::new(PUBSUB_TOPIC);
    let message = Message::new(pubsub_topic.clone(), "Hello World!".as_bytes());

    // 1. Init the publisher node.
    let keypair_publisher = Keypair::generate_ed25519();
    let mut publisher = new_floodsub_node(&keypair_publisher);

    println!("PUBLISHER > Peer ID: {}", publisher.local_peer_id());

    let listen_addr_publisher = PUBLISHER_LISTEN_MULTIADDDR.parse::<Multiaddr>().unwrap();
    publisher.listen_on(listen_addr_publisher.clone()).unwrap();

    println!("PUBLISHER > Listen address: {}", listen_addr_publisher);

    publisher
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");

    println!("PUBLISHER > Subscribed to topic: {}", pubsub_topic);

    // 2. Init the subscriber node.
    let keypair_subscriber = Keypair::generate_ed25519();
    let mut subscriber = new_floodsub_node(&keypair_subscriber);

    println!("SUBSCRIBER > Peer ID: {}", subscriber.local_peer_id());

    let listen_addr_subscriber = SUBSCRIBER_LISTEN_MULTIADDDR.parse::<Multiaddr>().unwrap();
    subscriber
        .listen_on(listen_addr_subscriber.clone())
        .expect("Failed to listen on address");

    println!("SUBSCRIBER > Listen address: {}", listen_addr_subscriber);

    subscriber
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");

    println!("SUBSCRIBER > Subscribed to topic: {}", pubsub_topic);

    // 3. Connect the two nodes.
    let dial_addr_subscriber = SUBSCRIBER_DIAL_MULTIADDDR.parse::<Multiaddr>().unwrap();
    publisher.dial(dial_addr_subscriber.clone()).unwrap();

    println!("PUBLISHER > Dialing address: {}", dial_addr_subscriber);

    let dial_addr_publisher = PUBLISHER_DIAL_MULTIADDDR.parse::<Multiaddr>().unwrap();
    subscriber.dial(dial_addr_publisher.clone()).unwrap();

    println!("SUBSCRIBER > Dialing address: {}", dial_addr_publisher);

    // 4. Create a task to publish the message after 2 seconds. Log all swarm events.
    let publish_fut = async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                    publisher.behaviour_mut().publish(message.clone()).unwrap();
                    println!("PUBLISHER > Published message (topic: {})", pubsub_topic);
                }
                event = publisher.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("PUBLISHER > New listen address: {}", address);
                    },
                    SwarmEvent::Dialing { peer_id, connection_id } => {
                        println!("PUBLISHER > Dialing peer: {:?} (connection: {})", peer_id, connection_id);
                    },
                    SwarmEvent::ConnectionEstablished { peer_id, connection_id, .. } => {
                        println!("PUBLISHER > Connection established with: {} (connection: {})", peer_id, connection_id);
                    },
                    _ => {},
                }
            }
        }
    };

    // 5. Create a task to listen to all published messages on the topic. Log all swarm events.
    //    Finish the task when the message is received.
    let subscriber_fut = async move {
        loop {
            let event = subscriber.select_next_some().await;
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("SUBSCRIBER > New listen address: {}", address);
                }
                SwarmEvent::Dialing {
                    peer_id,
                    connection_id,
                } => {
                    println!(
                        "SUBSCRIBER > Dialing peer: {:?} (connection: {})",
                        peer_id, connection_id
                    );
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    connection_id,
                    ..
                } => {
                    println!(
                        "SUBSCRIBER > Connection established with: {} (connection: {})",
                        peer_id, connection_id
                    );
                }
                SwarmEvent::Behaviour(event) => match event {
                    libp2p_pubsub_core::Event::MessageReceived { message, .. } => {
                        let msg_data = String::from_utf8_lossy(&message.data);
                        println!(
                            "SUBSCRIBER > Received message: {} (topic: {})",
                            msg_data, message.topic
                        );
                        return;
                    }
                },
                _ => {}
            }
        }
    };

    // 6. Poll the two futures until the message is received.
    tokio::select! {
        _ = publish_fut => {}
        _ = subscriber_fut => {
            exit(0);
        }
    }
}
