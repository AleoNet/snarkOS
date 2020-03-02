use crate::{
    context::Context,
    message::{
        types::{Ping, Version},
        Channel,
    },
    protocol::SyncHandler,
    server::Server,
    Handshake,
    Message,
};
use snarkos_consensus::{miner::MemoryPool, test_data::*};
use snarkos_storage::BlockStorage;

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

/// Starts a server on a new thread. Takes full ownership of server.
pub fn start_test_server(server: Server) {
    tokio::spawn(async move { server.listen().await.unwrap() });
}

/// Returns a tcp channel connected to the address
pub async fn connect_channel(listener: &mut TcpListener, address: SocketAddr) -> Channel {
    let channel = Channel::new_write_only(address).await.unwrap();
    let (reader, _socket) = listener.accept().await.unwrap();

    channel.update_reader(Arc::new(Mutex::new(reader)))
}

/// Returns the next tcp channel connected to the listener
pub async fn accept_channel(listener: &mut TcpListener, address: SocketAddr) -> Channel {
    let (reader, _peer) = listener.accept().await.unwrap();
    let channel = Channel::new_read_only(reader).unwrap();

    channel.update_writer(address).await.unwrap()
}

pub async fn do_handshake_get_channel(peer_address: SocketAddr, server_address: SocketAddr) -> Arc<Channel> {
    // Simulate message handler
    let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
    let (reader, _peer) = peer_listener.accept().await.unwrap();

    // Simulate Handshakes
    let channel = Channel::new_read_only(reader).unwrap();
    let (name, bytes) = channel.read().await.unwrap();
    assert_eq!(Version::name(), name);

    // Get final handshake with server
    let handshake = Handshake::receive_new(
        1u64,
        0u32,
        channel,
        Version::deserialize(bytes).unwrap(),
        server_address,
    )
    .await
    .unwrap();

    // return Arc::clone() of channel
    handshake.channel.clone()
}

/// Starts a fake node that accepts all tcp connections at the given socket address
pub async fn simulate_active_node(address: SocketAddr) {
    accept_all_messages(TcpListener::bind(address).await.unwrap());
}

/// Starts a fake node that accepts all tcp connections received by the given peer listener
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
        let channel = Arc::new(Channel::new_write_only(address).await.unwrap());
        channel.write(&Ping::new()).await.unwrap();
        tx.send(()).unwrap();
    });

    rx.await.unwrap();
    let channel = accept_channel(&mut listener, address).await;
    let (name, _bytes) = channel.read().await.unwrap();

    assert_eq!(Ping::name(), name);
}
