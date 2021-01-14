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

use crate::{
    consensus::{BLOCK_1, BLOCK_2, TRANSACTION_1},
    network::start_node,
};

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

    // T 0-2s: not much happens
    // T 2s: first sync occures, a peer isn't yet connected to sync with
    // T 4s: second sync occures, this time a peer is selected for the block sync
    sleep(Duration::new(5, 0)).await;

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

#[tokio::test]
async fn simple_transaction_sync() {
    use snarkos_consensus::memory_pool::Entry;
    use snarkvm_dpc::instantiated::Tx;
    use snarkvm_utilities::bytes::FromBytes;

    let node_alice = start_node(vec![]).await;
    let alice_address = node_alice.local_address().unwrap();

    // insert transaction into node_alice
    let mut memory_pool = node_alice.environment.memory_pool().lock();
    let storage = node_alice.environment.storage().read();

    let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
    let size = TRANSACTION_1.len();
    let entry = Entry {
        size_in_bytes: size,
        transaction: transaction.clone(),
    };

    memory_pool.insert(&storage, entry.clone()).unwrap().unwrap();

    // drop the locks to avoid deadlocks
    drop(memory_pool);
    drop(storage);

    let node_bob = start_node(vec![alice_address.to_string()]).await;

    // T 0-2s: not much happens
    // T 2s: first sync occures, a peer isn't yet connected to sync with
    // T 4s: second sync occures, this time a peer is selected for the block sync
    sleep(Duration::new(5, 0)).await;

    // check transaction is present in bob's memory pool
    assert!(node_bob.environment.memory_pool().lock().contains(&entry));
}
