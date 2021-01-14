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

pub mod tcp;
pub use tcp::*;

pub mod blocks;
pub use blocks::*;

pub mod sync;

use crate::{
    consensus::{FIXTURE_VK, TEST_CONSENSUS},
    dpc::load_verifying_parameters,
};

use snarkos_network::{external::Channel, Environment, Server};

use parking_lot::{Mutex, RwLock};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::{tcp::OwnedReadHalf, TcpListener};

/// Starts a node with the specified bootnodes.
pub async fn start_node(bootnodes: Vec<String>) -> Server {
    let storage = FIXTURE_VK.ledger();
    let memory_pool = snarkos_consensus::MemoryPool::new();
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
    let consensus = TEST_CONSENSUS.clone();
    let parameters = load_verifying_parameters();
    let socket_address = None;
    let min_peers = 1;
    let max_peers = 10;
    let is_bootnode = false;
    let is_miner = false;

    let environment = Environment::new(
        Arc::new(RwLock::new(storage)),
        memory_pool_lock,
        Arc::new(consensus),
        Arc::new(parameters),
        socket_address,
        min_peers,
        max_peers,
        bootnodes,
        is_bootnode,
        is_miner,
        Duration::from_secs(2),
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
    .unwrap();

    let mut node = Server::new(environment).await.unwrap();
    node.start().await.unwrap();

    node
}

/// Puts the current tokio thread to sleep for given milliseconds
pub async fn sleep(time: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(time)).await;
}

/// Starts a server on a new thread. Takes full ownership of server.
pub fn start_test_server(mut server: Server) {
    tokio::spawn(async move { server.start().await.unwrap() });
}

/// Returns the next tcp channel connected to the listener
pub async fn accept_channel(listener: &mut TcpListener) -> (Channel, OwnedReadHalf) {
    let (stream, address) = listener.accept().await.unwrap();
    Channel::new(address, stream)
}

/// Starts a fake node that accepts all tcp connections at the given socket address
pub async fn simulate_active_node() -> SocketAddr {
    let (addr, listener) = random_bound_address().await;
    accept_all_messages(listener);
    addr
}

/// Starts a fake node that accepts all tcp connections received by the given peer listener
pub fn accept_all_messages(peer_listener: TcpListener) {
    tokio::spawn(async move {
        loop {
            peer_listener.accept().await.unwrap();
        }
    });
}
