use crate::{
    base::{handshake_request, handshake_response, Context, Message},
    Server,
    SyncHandler,
};
use snarkos_consensus::{miner::MemoryPool, test_data::*};
use snarkos_storage::BlockStorage;

use rand::Rng;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    prelude::*,
    sync::Mutex,
};

pub const LOCALHOST: &'static str = "127.0.0.1:";
pub const LOCALHOST_SERVER: &'static str = "127.0.0.1:";
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

/// Starts a server on a new thread. Takes full ownership of server.
pub fn start_test_server(server: Server) {
    tokio::spawn(async move { server.listen().await.unwrap() });
}

/// Returns the next message received by the given peer listener
pub async fn get_next_message(peer_listener: &mut TcpListener) -> Message {
    let (mut stream, _) = peer_listener.accept().await.unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    bincode::deserialize(&buf[0..n]).unwrap()
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
    {
        let mut stream = TcpStream::connect(&address).await.unwrap();
        let ping = bincode::serialize("ping").unwrap();
        let _result = stream.write(&ping).await.unwrap();
    }

    let (mut stream, _) = listener.accept().await.unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();

    let actual_message: String = bincode::deserialize(&buf[0..n]).unwrap();

    assert_eq!(n, 12);
    assert_eq!(actual_message, "ping");
}

/// Complete a full handshake between a server and peer
pub async fn peer_server_handshake(peer_address: SocketAddr, server_address: SocketAddr) {
    // 1. Start peer server

    let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
    sleep(100).await;

    // 2. Initiate handshake request from peer to server

    handshake_request(1, server_address).await.unwrap();
    sleep(100).await;

    // 3. Check that server sent a Verack message

    let actual = get_next_message(&mut peer_listener).await;
    let expected = Message::Verack;
    assert_eq!(actual, expected);

    // 4. Check that server sent a Version message

    get_next_message(&mut peer_listener).await;

    // 5. Initiate handshake response from peer to server

    handshake_response(1, server_address, false).await.unwrap();

    drop(peer_listener);
    sleep(100).await;
}
