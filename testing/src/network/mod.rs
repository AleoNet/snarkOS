// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::consensus::*;
use snarkos_consensus::{MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::PublicParameters};
use snarkos_network::{
    external::{Channel, SyncHandler},
    server::{Context, Server},
};

use rand::Rng;
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};

pub const LOCALHOST: &'static str = "0.0.0.0:";
pub const CONNECTION_FREQUENCY_LONG: u64 = 100000; // 100 seconds
pub const CONNECTION_FREQUENCY_SHORT: u64 = 100; // .1 seconds
pub const CONNECTION_FREQUENCY_SHORT_TIMEOUT: u64 = 200; // .2 seconds

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
    storage: Arc<MerkleTreeLedger>,
    parameters: PublicParameters<Components>,
    connection_frequency: u64,
) -> Server {
    let consensus = TEST_CONSENSUS.clone();
    let memory_pool = MemoryPool::new();
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

    let sync_handler = SyncHandler::new(bootnode_address);
    let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

    Server::new(
        Context::new(server_address, 5, 1, 10, true, vec![]),
        consensus,
        storage,
        parameters,
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
