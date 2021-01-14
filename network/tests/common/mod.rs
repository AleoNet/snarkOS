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

use snarkos_network::{Environment, Server};
use snarkos_testing::{
    consensus::{FIXTURE_VK, TEST_CONSENSUS},
    dpc::load_verifying_parameters,
};

use std::sync::Arc;

use parking_lot::lock_api::{Mutex, RwLock};

/// Starts a node with the specified bootnodes.
pub async fn start_node(bootnodes: Vec<String>) -> Server {
    let storage = FIXTURE_VK.ledger();
    let memory_pool = snarkos_consensus::MemoryPool::new();
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
    let consensus = TEST_CONSENSUS.clone();
    let parameters = load_verifying_parameters();
    let socket_address = None;
    let min_peers = 2;
    let max_peers = 50;
    let sync_interval = 10;
    let mempool_interval = 5;
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
        sync_interval,
        mempool_interval,
        // TODO: these should probably be a 'Vec<SocketAddr>'.
        bootnodes,
        is_bootnode,
        is_miner,
    )
    .unwrap();

    let mut node = Server::new(environment).await.unwrap();
    node.start().await.unwrap();
    node
}
