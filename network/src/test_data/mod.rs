use crate::{context::Context, protocol::SyncHandler, server::Server};
use snarkos_consensus::{miner::MemoryPool, test_data::*};
use snarkos_storage::BlockStorage;

use crate::message::{types::Ping, Channel, MessageName};
use rand::Rng;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{oneshot, Mutex},
};

pub const ALEO_PORT: &'static str = "4130";
pub const LOCALHOST: &'static str = "127.0.0.1:";
pub const LOCALHOST_PEER: &'static str = "127.0.0.2:";
pub const LOCALHOST_BOOTNODE: &'static str = "127.0.0.3:";
pub const CONNECTION_FREQUENCY_LONG: u64 = 100000; // 100 seconds
pub const CONNECTION_FREQUENCY_SHORT: u64 = 100; // .1 seconds
pub const CONNECTION_FREQUENCY_SHORT_TIMEOUT: u64 = 200; // .2 seconds

///// Returns a tcp listener to the aleo server port on localhost
//pub async fn aleo_listener() -> TcpListener {
//    TcpListener::bind(format!("{}{}", LOCALHOST, ALEO_SERVER_PORT)).await.unwrap()
//}

///// Returns a random server tcp socket address
//pub fn random_server_address() -> SocketAddr {
//    let mut rng = rand::thread_rng();
//    let string = format!("{}{}", LOCALHOST_SERVER, rng.gen_range(1023, 9999));
//    string.parse::<SocketAddr>().unwrap()
//}

///// Returns a random peer tcp socket address
//pub fn random_peer_address() -> SocketAddr {
//    let mut rng = rand::thread_rng();
//    let string = format!("{}{}", LOCALHOST_PEER, rng.gen_range(1023, 9999));
//    string.parse::<SocketAddr>().unwrap()
//}
//
///// Returns a random bootnode tcp socket address
//pub fn random_bootnode_address() -> SocketAddr {
//    let mut rng = rand::thread_rng();
//    let string = format!("{}{}", LOCALHOST_BOOTNODE, rng.gen_range(1023, 9999));
//    string.parse::<SocketAddr>().unwrap()
//}
//

/// Returns a socket address from the aleo server port on localhost
pub fn aleo_socket_address() -> SocketAddr {
    let string = format!("{}{}", LOCALHOST, ALEO_PORT);
    string.parse::<SocketAddr>().unwrap()
}

/// Returns a random tcp socket address
pub fn random_socket_address() -> SocketAddr {
    let mut rng = rand::thread_rng();
    let string = format!("{}{}", LOCALHOST, rng.gen_range(1023, 9999));
    string.parse::<SocketAddr>().unwrap()
}

/// Puts the current tokio thread to sleep for given milliseconds
pub async fn sleep(time: u64) {
    tokio::time::delay_for(std::time::Duration::from_millis(time)).await;
}

/// Returns a server struct with given argumnets
pub fn initialize_test_server(
    server_address: SocketAddr,
    bootnode_address: SocketAddr,
    storage: Arc<BlockStorage>,
    connection_frequency: u64,
) -> Server {
    let consensus = TEST_CONSENSUS;
    let memory_pool = MemoryPool::new();
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

    let sync_handler = SyncHandler::new(bootnode_address);
    let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

    Server::new(
        Context::new(server_address, 5, 1, 10, true, vec![]),
        consensus,
        storage,
        memory_pool_lock,
        sync_handler_lock,
        connection_frequency,
    )
}

///// Adds Peer to peerbook and connection handler
//pub fn initialize_test_context(server_address: SocketAddr, peer_address: SocketAddr) -> Context {
//    let context = Context::new(server_address, 5, 1, 10, true, vec![])
//
//}

/// Starts a server on a new thread. Takes full ownership of server.
pub fn start_test_server(server: Server) {
    tokio::spawn(async move { server.listen().await.unwrap() });
}

/// Returns the next tcp channel connected to the listener
pub async fn get_next_channel(listener: &mut TcpListener) -> Arc<Channel> {
    let (stream, peer_address) = listener.accept().await.unwrap();
    Arc::new(Channel::new(stream, peer_address).await.unwrap())
}

/// Starts a fake node that accepts all messages at the given socket address
pub async fn simulate_active_node(address: SocketAddr) {
    accept_all_messages(TcpListener::bind(address).await.unwrap());
}

/// Starts a fake node that accepts all messages received by the given peer listener
pub fn accept_all_messages(mut peer_listener: TcpListener) {
    tokio::spawn(async move {
        loop {
            peer_listener.accept().await.unwrap();
        }
    });
}

/// Send a dummy message to the peer and make sure no other messages were received
pub async fn ping(address: SocketAddr, mut listener: TcpListener) {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let channel = Arc::new(Channel::connect(address).await.unwrap());
        channel.write(&Ping::new()).await.unwrap();
        tx.send(()).unwrap();
    });

    rx.await.unwrap();
    println!("waited");
    let channel = get_next_channel(&mut listener).await;
    println!("getting next channel");
    let (name, _bytes) = channel.read().await.unwrap();

    assert_eq!(MessageName::from("ping"), name);
}

///// Complete a full handshake between a server and peer
//pub async fn peer_server_handshake(peer_address: SocketAddr, server_address: SocketAddr) {
//    // 1. Start peer server
//
//    let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//    sleep(100).await;
//
//    // 2. Initiate handshake request from peer to server
//
//    handshake_request(1, server_address).await.unwrap();
//    sleep(100).await;
//
//    // 3. Check that server sent a Verack message
//
//    let actual = get_next_message(&mut peer_listener).await;
//    let expected = Message::Verack;
//    assert_eq!(actual, expected);
//
//    // 4. Check that server sent a Version message
//
//    get_next_message(&mut peer_listener).await;
//
//    // 5. Initiate handshake response from peer to server
//
//    handshake_response(1, server_address, false).await.unwrap();
//
//    drop(peer_listener);
//    sleep(100).await;
//}
