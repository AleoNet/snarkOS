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
    consensus::{BLOCK_1, BLOCK_2, FIXTURE_VK, TEST_CONSENSUS},
    dpc::load_verifying_parameters,
};
use snarkvm_objects::block::Block;

use std::{sync::Arc, time::Duration};

use parking_lot::lock_api::{Mutex, RwLock};
use tokio::time::sleep;

/// Starts a node with the specified bootnodes.
async fn start_node(bootnodes: Vec<String>) -> Server {
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

#[tokio::test]
async fn simple_block_sync() {
    let filter = tracing_subscriber::EnvFilter::from_default_env().add_directive("tokio_reactor=off".parse().unwrap());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let node_alice = start_node(vec![]).await;
    let alice_address = node_alice.local_address().unwrap();

    // insert blocks into node_alice
    let block_1 = BLOCK_1.to_vec();
    let block_struct_1 = Block::deserialize(&block_1).unwrap();
    node_alice
        .environment
        .consensus_parameters()
        .receive_block(
            node_alice.environment.dpc_parameters(),
            &node_alice.environment.storage().read(),
            &mut node_alice.environment.memory_pool().lock(),
            &block_struct_1,
        )
        .unwrap();

    let block_2 = BLOCK_2.to_vec();
    let block_struct_2 = Block::deserialize(&block_2).unwrap();
    node_alice
        .environment
        .consensus_parameters()
        .receive_block(
            node_alice.environment.dpc_parameters(),
            &node_alice.environment.storage().read(),
            &mut node_alice.environment.memory_pool().lock(),
            &block_struct_2,
        )
        .unwrap();

    let node_bob = start_node(vec![alice_address.to_string()]).await;

    // T 0-10s: not much happens
    // T 11s: first sync occures, a peer isn't yet connected to sync with
    // T 21s: second sync occures, this time a peer is selected for the block sync
    sleep(Duration::new(22, 0)).await;

    // check blocks present in alice's chain were synced to bob's
    assert!(
        node_bob
            .environment
            .storage()
            .read()
            .block_hash_exists(&block_struct_1.header.get_hash())
    );

    assert!(
        node_bob
            .environment
            .storage()
            .read()
            .block_hash_exists(&block_struct_2.header.get_hash())
    );
}
