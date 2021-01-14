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
mod common;
use common::start_node;

use snarkos_testing::consensus::{BLOCK_1, BLOCK_2};
use snarkvm_objects::block::Block;

use std::time::Duration;

use tokio::time::sleep;

#[tokio::test]
async fn simple_block_sync() {
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
