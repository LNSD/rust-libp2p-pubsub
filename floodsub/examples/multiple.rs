use std::process::exit;

use futures::StreamExt;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::core::upgrade;
use libp2p::identity::Keypair;
use libp2p::plaintext::PlainText2Config;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{dns, tcp, yamux, Multiaddr, PeerId, Swarm, Transport};

use floodsub::Protocol as Floodsub;
use pubsub::{Behaviour, Config, IdentTopic, Message};

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

/// Set up a subscriber node.
fn setup_subscriber(sub: char, keypair: &Keypair, listen_addr: &str) -> Swarm<Behaviour<Floodsub>> {
    let mut subscriber = new_floodsub_node(keypair);

    println!("SUBSCRIBER {sub} > Peer ID: {}", subscriber.local_peer_id());

    let listen_addr = listen_addr.parse::<Multiaddr>().unwrap();
    subscriber.listen_on(listen_addr.clone()).unwrap();

    println!("SUBSCRIBER {sub} > Listen address: {}", listen_addr);

    subscriber
}

/// Create a async task for the subscriber node that resolves once the message is received.
async fn new_subscriber_task(sub: char, mut subscriber: Swarm<Behaviour<Floodsub>>) {
    loop {
        let event = subscriber.select_next_some().await;
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("SUBSCRIBER {sub} > New listen address: {}", address);
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                println!(
                    "SUBSCRIBER {sub} > Connection established with: {} (connection: {})",
                    peer_id, connection_id
                );
            }
            SwarmEvent::Behaviour(event) => match event {
                pubsub::Event::MessageReceived { message, .. } => {
                    let msg_data = String::from_utf8_lossy(message.data());
                    println!(
                        "SUBSCRIBER {sub} > Received message: {} (topic: {})",
                        msg_data,
                        message.topic()
                    );
                    return;
                }
            },
            _ => {}
        }
    }
}

/// The topic to publish/subscribe to.
const PUBSUB_TOPIC: &str = "/examples/simple-topic";

/// The network noes addresses.
const PUBLISHER_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/60200";
const PUBLISHER_DIAL_ADDR: &str = "/dns4/localhost/tcp/60200";

const RELAY_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/60201";
const RELAY_DIAL_ADDR: &str = "/dns4/localhost/tcp/60201";

const SUBSCRIBER_A_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/60211";
const SUBSCRIBER_B_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/60212";
const SUBSCRIBER_C_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/60213";

/// An example where multiple nodes connect forming a mesh of nodes subscribed to the same topic.
/// The publisher sends a message, and an intermediate node and the subscribers receive it.
/// ```
///                                       ┌──────────────┐
///                                   ┌──►│ SUBSCRIBER A │
///                                   │   └──────────────┘
///   ┌───────────┐     ┌─────────┐   │   ┌──────────────┐
///   │ PUBLISHER │◄───►│  RELAY  │◄──┼──►│ SUBSCRIBER B │
///   └───────────┘     └─────────┘   │   └──────────────┘
///                                   │   ┌──────────────┐
///                                   └──►│ SUBSCRIBER C │
///                                       └──────────────┘
/// ```
#[tokio::main]
async fn main() {
    // Create a message to publish with the topic.
    let pubsub_topic = IdentTopic::new(PUBSUB_TOPIC);
    let message = Message::new(pubsub_topic.clone(), "Hello World!".as_bytes());

    // 1. Init the publisher node.
    let keypair_publisher = Keypair::generate_ed25519();
    let mut publisher = new_floodsub_node(&keypair_publisher);

    println!("PUBLISHER > Peer ID: {}", publisher.local_peer_id());

    let listen_addr_publisher = PUBLISHER_LISTEN_ADDR.parse::<Multiaddr>().unwrap();
    publisher.listen_on(listen_addr_publisher.clone()).unwrap();

    println!("PUBLISHER > Listen address: {}", listen_addr_publisher);

    // 2. Init the relay node.
    let keypair_relay = Keypair::generate_ed25519();
    let mut relay = new_floodsub_node(&keypair_relay);

    println!("RELAY > Peer ID: {}", relay.local_peer_id());

    let listen_addr_subscriber = RELAY_LISTEN_ADDR.parse::<Multiaddr>().unwrap();
    relay
        .listen_on(listen_addr_subscriber.clone())
        .expect("Failed to listen on address");

    println!("RELAY > Listen address: {}", listen_addr_subscriber);

    // 3. Setup the subscribers
    let keypair_subscriber_a = Keypair::generate_ed25519();
    let mut subscriber_a = setup_subscriber('A', &keypair_subscriber_a, SUBSCRIBER_A_LISTEN_ADDR);

    let keypair_subscriber_b = Keypair::generate_ed25519();
    let mut subscriber_b = setup_subscriber('B', &keypair_subscriber_b, SUBSCRIBER_B_LISTEN_ADDR);

    let keypair_subscriber_c = Keypair::generate_ed25519();
    let mut subscriber_c = setup_subscriber('C', &keypair_subscriber_c, SUBSCRIBER_C_LISTEN_ADDR);

    // 4. Subscribe all nodes to the topic.
    publisher
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");
    relay
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");
    subscriber_a
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");
    subscriber_b
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");
    subscriber_c
        .behaviour_mut()
        .subscribe(pubsub_topic.clone())
        .expect("Failed to subscribe to topic");

    // 5. Connect the two nodes.
    let dial_addr_publisher = PUBLISHER_DIAL_ADDR.parse::<Multiaddr>().unwrap();
    relay.dial(dial_addr_publisher.clone()).unwrap();

    let dial_addr_relay = RELAY_DIAL_ADDR.parse::<Multiaddr>().unwrap();
    subscriber_a.dial(dial_addr_relay.clone()).unwrap();
    subscriber_b.dial(dial_addr_relay.clone()).unwrap();
    subscriber_c.dial(dial_addr_relay.clone()).unwrap();

    // 6. Create a task to publish the message after 2 seconds. Log all swarm events.
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
                    SwarmEvent::ConnectionEstablished { peer_id, connection_id, .. } => {
                        println!("PUBLISHER > Connection established with: {} (connection: {})", peer_id, connection_id);
                    },
                    _ => {},
                }
            }
        }
    };

    // 7. Create a task to listen to all published messages on the topic. Log all swarm events.
    let relay_fut = async move {
        loop {
            let event = relay.select_next_some().await;
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("RELAY > New listen address: {}", address);
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    connection_id,
                    ..
                } => {
                    println!(
                        "RELAY > Connection established with: {} (connection: {})",
                        peer_id, connection_id
                    );
                }
                SwarmEvent::Behaviour(event) => match event {
                    pubsub::Event::MessageReceived { message, .. } => {
                        let msg_data = String::from_utf8_lossy(message.data());
                        println!(
                            "RELAY > Received message: {} (topic: {})",
                            msg_data,
                            message.topic()
                        );
                    }
                },
                _ => {}
            }
        }
    };

    // 8. Create a task to listen to all published messages on the topic. Log all swarm events.
    //    Finish the task when all subscriber have received the message.
    let subscribers_fut = futures::future::join_all([
        new_subscriber_task('A', subscriber_a),
        new_subscriber_task('B', subscriber_b),
        new_subscriber_task('C', subscriber_c),
    ]);

    // 9. Poll the two futures until the message is received.
    tokio::select! {
        _ = publish_fut => {}
        _ = relay_fut => {}
        _ = subscribers_fut => {
            exit(0);
        }
    }
}
