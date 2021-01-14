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

use snarkos_testing::consensus::TRANSACTION_1;

use std::time::Duration;

use tokio::time::sleep;

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

    // T 0-10s: not much happens
    // T 11s: first sync occures, a peer isn't yet connected to sync with
    // T 21s: second sync occures, this time a peer is selected for the block sync
    sleep(Duration::new(22, 0)).await;

    // check transaction is present in bob's memory pool
    assert!(node_bob.environment.memory_pool().lock().contains(&entry));
}
