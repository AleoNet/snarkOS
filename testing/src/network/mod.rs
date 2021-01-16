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

pub mod blocks;
pub use blocks::*;

#[cfg(test)]
pub mod sync;

use crate::{
    consensus::{FIXTURE_VK, TEST_CONSENSUS},
    dpc::load_verifying_parameters,
};

use snarkos_consensus::MerkleTreeLedger;
use snarkos_network::{Environment, Server};
use snarkvm_dpc::{instantiated::Components, PublicParameters};

use parking_lot::{Mutex, RwLock};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;

/// Returns a random tcp socket address and binds it to a listener
pub async fn random_bound_address() -> (SocketAddr, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (addr, listener)
}

/// Returns an `Environment` struct with given arguments
pub fn test_environment(
    socket_address: Option<SocketAddr>,
    bootnodes: Vec<String>,
    storage: Arc<RwLock<MerkleTreeLedger>>,
    parameters: PublicParameters<Components>,
) -> Environment {
    let memory_pool = snarkos_consensus::MemoryPool::new();
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
    let consensus = TEST_CONSENSUS.clone();
    let min_peers = 1;
    let max_peers = 10;
    let is_bootnode = false;
    let is_miner = false;

    Environment::new(
        storage,
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
    .unwrap()
}

/// Starts a node with the specified bootnodes.
pub async fn start_node(bootnodes: Vec<String>) -> Server {
    let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
    let environment = test_environment(None, bootnodes, storage, load_verifying_parameters());

    let mut node = Server::new(environment).await.unwrap();
    node.start().await.unwrap();

    node
}
