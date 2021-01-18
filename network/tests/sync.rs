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
use common::{handshake, write_message_to_stream};

use snarkos_consensus::memory_pool::Entry;
use snarkos_network::external::message::*;
use snarkos_testing::{
    consensus::{BLOCK_1, BLOCK_1_HEADER_HASH, BLOCK_2, BLOCK_2_HEADER_HASH, TRANSACTION_1, TRANSACTION_2},
    network::{read_header, read_payload},
};

use snarkvm_objects::block_header_hash::BlockHeaderHash;
use snarkvm_utilities::bytes::FromBytes;

use std::time::Duration;

use tokio::time::sleep;

#[tokio::test]
async fn block_sync_initiator_side() {
    // handshake between the fake node and full node
    let (node, mut peer_stream) =
        handshake(Duration::from_secs(10), Duration::from_secs(2), Duration::from_secs(10)).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 64];

    // check GetSync message was received
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::GetSync(..)));

    let block_1_header_hash = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
    let block_2_header_hash = BlockHeaderHash::new(BLOCK_2_HEADER_HASH.to_vec());

    let block_header_hashes = vec![block_1_header_hash.clone(), block_2_header_hash.clone()];

    // respond to GetSync with Sync message containing the block header hashes of the missing
    // blocks
    let sync = Payload::Sync(block_header_hashes);
    write_message_to_stream(sync, &mut peer_stream).await;

    // make sure both GetBlock messages are received
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    let block_hash = if let Payload::GetBlock(block_hash) = bincode::deserialize(&payload).unwrap() {
        block_hash
    } else {
        unreachable!();
    };

    assert_eq!(block_hash, block_1_header_hash);

    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    let block_hash = if let Payload::GetBlock(block_hash) = bincode::deserialize(&payload).unwrap() {
        block_hash
    } else {
        unreachable!();
    };

    assert_eq!(block_hash, block_2_header_hash);

    // respond with the full blocks
    let block_1 = Payload::Block(BLOCK_1.to_vec());
    write_message_to_stream(block_1, &mut peer_stream).await;

    let block_2 = Payload::Block(BLOCK_2.to_vec());
    write_message_to_stream(block_2, &mut peer_stream).await;

    sleep(Duration::from_millis(200)).await;

    // check the blocks have been added to the node's chain
    assert!(
        node.environment
            .storage()
            .read()
            .block_hash_exists(&block_1_header_hash)
    );

    assert!(
        node.environment
            .storage()
            .read()
            .block_hash_exists(&block_2_header_hash)
    );
}

#[tokio::test]
async fn block_sync_responder_side() {
    // handshake between the fake and full node
    let (node, mut peer_stream) = handshake(
        Duration::from_secs(10),
        Duration::from_secs(10),
        Duration::from_secs(10),
    )
    .await;

    // insert block into node
    let block_struct_1 = snarkvm_objects::Block::deserialize(&BLOCK_1).unwrap();
    node.environment
        .consensus_parameters()
        .receive_block(
            node.environment.dpc_parameters(),
            &node.environment.storage().read(),
            &mut node.environment.memory_pool().lock(),
            &block_struct_1,
        )
        .unwrap();

    // send a GetSync with an empty vec as only the genesis block is in the ledger
    let get_sync = Payload::GetSync(vec![]);
    write_message_to_stream(get_sync, &mut peer_stream).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 4096];

    // receive a Sync message from the node with the block header
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    let sync = if let Payload::Sync(sync) = bincode::deserialize(&payload).unwrap() {
        sync
    } else {
        unreachable!();
    };

    let block_header_hash = sync.first().unwrap();

    // check it matches the block inserted into the node's ledger
    assert_eq!(*block_header_hash, block_struct_1.header.get_hash());

    // request the block from the node
    let get_block = Payload::GetBlock(block_header_hash.clone());
    write_message_to_stream(get_block, &mut peer_stream).await;

    // receive a SyncBlock message with the requested block
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    let block = if let Payload::SyncBlock(block) = bincode::deserialize(&payload).unwrap() {
        block
    } else {
        unreachable!();
    };
    let block = snarkvm_objects::Block::deserialize(&block).unwrap();

    assert_eq!(block, block_struct_1);
}

#[tokio::test]
async fn transaction_sync_initiator_side() {
    // handshake between the fake node and full node
    let (node, mut peer_stream) =
        handshake(Duration::from_secs(10), Duration::from_secs(10), Duration::from_secs(2)).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 64];

    // check GetMemoryPool message was received
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    assert!(matches!(
        bincode::deserialize(&payload).unwrap(),
        Payload::GetMemoryPool
    ));

    // Respond with MemoryPool message
    let memory_pool = Payload::MemoryPool(vec![TRANSACTION_1.to_vec(), TRANSACTION_2.to_vec()]);
    write_message_to_stream(memory_pool, &mut peer_stream).await;

    // Create the entries to verify
    let size = TRANSACTION_1.len();
    let entry_1 = Entry {
        size_in_bytes: size,
        transaction: Tx::read(&TRANSACTION_1[..]).unwrap(),
    };

    let size = TRANSACTION_2.len();
    let entry_2 = Entry {
        size_in_bytes: size,
        transaction: Tx::read(&TRANSACTION_2[..]).unwrap(),
    };

    sleep(Duration::from_millis(200)).await;

    // Verify the transactions have been stored in the node's memory pool
    assert!(node.environment.memory_pool().lock().contains(&entry_1));
    assert!(node.environment.memory_pool().lock().contains(&entry_2));
}

#[tokio::test]
async fn transaction_sync_responder_side() {
    // handshake between the fake node and full node
    let (node, mut peer_stream) = handshake(
        Duration::from_secs(10),
        Duration::from_secs(10),
        Duration::from_secs(10),
    )
    .await;

    // insert transaction into node
    let mut memory_pool = node.environment.memory_pool().lock();
    let storage = node.environment.storage().read();

    let entry_1 = Entry {
        size_in_bytes: TRANSACTION_1.len(),
        transaction: Tx::read(&TRANSACTION_1[..]).unwrap(),
    };

    let entry_2 = Entry {
        size_in_bytes: TRANSACTION_2.len(),
        transaction: Tx::read(&TRANSACTION_2[..]).unwrap(),
    };

    memory_pool.insert(&storage, entry_1).unwrap().unwrap();
    memory_pool.insert(&storage, entry_2).unwrap().unwrap();

    // drop the locks to avoid deadlocks
    drop(memory_pool);
    drop(storage);

    // send a GetMemoryPool message
    let get_memory_pool = Payload::GetMemoryPool;
    write_message_to_stream(get_memory_pool, &mut peer_stream).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 4096];

    // check GetMemoryPool message was received
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    let txs = if let Payload::MemoryPool(txs) = bincode::deserialize(&payload).unwrap() {
        txs
    } else {
        unreachable!();
    };

    // check transactions
    assert!(txs.contains(&TRANSACTION_1.to_vec()));
    assert!(txs.contains(&TRANSACTION_2.to_vec()));
}
