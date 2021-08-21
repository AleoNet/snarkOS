// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use snarkos_storage::{BlockStatus, Digest, VMBlock};
use snarkvm_utilities::to_bytes_le;
use tokio::time::sleep;

use crate::{
    network::{handshaken_node_and_peer, test_node, ConsensusSetup, TestSetup},
    sync::{BLOCK_1, BLOCK_1_HEADER_HASH, BLOCK_2, BLOCK_2_HEADER_HASH, TRANSACTION_2},
    wait_until,
};

use snarkos_network::message::*;

use snarkvm_dpc::{block_header_hash::BlockHeaderHash, testnet1::instantiated::Testnet1Transaction, Block};
#[cfg(test)]
use snarkvm_utilities::ToBytes;

use std::time::Duration;

#[tokio::test]
async fn block_initiator_side() {
    // handshake between a fake node and a full node
    let setup = TestSetup {
        consensus_setup: Some(ConsensusSetup {
            block_sync_interval: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    let (node, mut peer) = handshaken_node_and_peer(setup).await;

    // check if the peer has received an automatic Ping message from the node
    wait_until!(5, {
        loop {
            let payload = peer.read_payload().await;
            if matches!(payload, Ok(Payload::Ping(..))) {
                break true;
            }
        }
    });

    // wait for the block_sync_interval to "expire"
    sleep(Duration::from_secs(1)).await;

    // trigger the full node to request synchronization by sending it a higher block_height than it has
    let ping = Payload::Ping(2u32);
    peer.write_message(&ping).await;

    // read the Pong
    wait_until!(5, {
        let payload = peer.read_payload().await.unwrap();
        matches!(payload, Payload::Pong)
    });

    // check if a GetSync message was received
    wait_until!(5, {
        let payload = peer.read_payload().await.unwrap();
        matches!(payload, Payload::GetSync(..))
    });

    let block_1_header_hash = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
    let block_2_header_hash = BlockHeaderHash::new(BLOCK_2_HEADER_HASH.to_vec());

    let block_header_hashes = vec![block_1_header_hash.clone(), block_2_header_hash.clone()];

    // respond to GetSync with Sync message containing the block header hashes of the missing
    // blocks
    let sync = Payload::Sync(block_header_hashes);
    peer.write_message(&sync).await;

    // make sure both GetBlock messages are received
    let payload = peer.read_payload().await.unwrap();
    let block_hashes = if let Payload::GetBlocks(block_hashes) = payload {
        block_hashes
    } else {
        unreachable!();
    };

    assert!(block_hashes.contains(&block_1_header_hash) && block_hashes.contains(&block_2_header_hash));

    // respond with the full blocks
    let block_1 = Payload::SyncBlock(to_bytes_le![&*BLOCK_1].unwrap(), Some(1));
    peer.write_message(&block_1).await;

    let block_2 = Payload::SyncBlock(to_bytes_le![&*BLOCK_2].unwrap(), Some(2));
    peer.write_message(&block_2).await;

    // check the blocks have been added to the node's chain
    wait_until!(
        5,
        matches!(
            node.storage
                .get_block_state(&block_1_header_hash.0.into())
                .await
                .unwrap(),
            BlockStatus::Committed(_)
        )
    );
    wait_until!(
        1,
        matches!(
            node.storage
                .get_block_state(&block_2_header_hash.0.into())
                .await
                .unwrap(),
            BlockStatus::Committed(_)
        )
    );
}

#[tokio::test]
async fn block_responder_side() {
    // handshake between a fake node and a full node
    let (node, mut peer) = handshaken_node_and_peer(TestSetup::default()).await;

    // check if the peer has received an automatic Ping message from the node
    wait_until!(5, {
        let payload = peer.read_payload().await.unwrap();
        matches!(payload, Payload::Ping(..))
    });

    // insert block into node
    let block_struct_1 = BLOCK_1.clone();
    assert!(node.expect_sync().consensus.receive_block(block_struct_1.clone()).await);

    // send a GetSync with an empty vec as only the genesis block is in the ledger
    let get_sync = Payload::GetSync(vec![]);
    peer.write_message(&get_sync).await;

    // receive a Sync message from the node with the block header
    let payload = peer.read_payload().await.unwrap();
    let sync = if let Payload::Sync(sync) = payload {
        sync
    } else {
        unreachable!();
    };

    let block_header_hash = sync.get(1).unwrap();
    let block_header_hash_digest: Digest = block_header_hash.0.into();
    // check it matches the block inserted into the node's ledger
    assert_eq!(block_header_hash_digest, block_struct_1.header.hash());

    // request the block from the node
    let get_block = Payload::GetBlocks(vec![block_header_hash.clone()]);
    peer.write_message(&get_block).await;

    // receive a SyncBlock message with the requested block
    let payload = peer.read_payload().await.unwrap();
    let block = if let Payload::SyncBlock(block, Some(1)) = payload {
        block
    } else {
        unreachable!();
    };
    let block: Block<Testnet1Transaction> = snarkvm_dpc::Block::deserialize(&block).unwrap();
    let block = <_ as VMBlock>::serialize(&block).unwrap();

    assert_eq!(block, block_struct_1);
}

#[test]
#[ignore]
fn block_propagation() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let setup = TestSetup {
        consensus_setup: Some(ConsensusSetup {
            is_miner: true,
            ..Default::default()
        }),
        tokio_handle: Some(rt.handle().clone()),
        ..Default::default()
    };

    rt.block_on(async move {
        let (_node, mut peer) = handshaken_node_and_peer(setup).await;

        wait_until!(60, {
            let payload = peer.read_payload().await.unwrap();
            matches!(payload, Payload::Block(..))
        });
    });
}

#[tokio::test]
#[ignore]
async fn block_two_node() {
    let setup = TestSetup {
        peer_sync_interval: 1,
        ..Default::default()
    };
    let node_alice = test_node(setup).await;
    let alice_address = node_alice.local_address().unwrap();

    const NUM_BLOCKS: usize = 100;

    let blocks = crate::sync::TestBlocks::load(Some(NUM_BLOCKS), "test_blocks_100_1").0;
    assert_eq!(blocks.len(), NUM_BLOCKS);

    for block in blocks {
        assert!(node_alice.expect_sync().consensus.receive_block(block).await);
    }

    let setup = TestSetup {
        consensus_setup: Some(ConsensusSetup {
            block_sync_interval: 5,
            ..Default::default()
        }),
        peer_sync_interval: 5,
        bootnodes: vec![alice_address.to_string()],
        ..Default::default()
    };
    let node_bob = test_node(setup).await;

    // check blocks present in alice's chain were synced to bob's
    wait_until!(30, node_bob.storage.canon().await.unwrap().block_height == NUM_BLOCKS);
}

#[tokio::test]
async fn transaction_initiator_side() {
    // tracing_subscriber::fmt::init();
    // handshake between a fake node and a full node
    let setup = TestSetup {
        consensus_setup: Some(ConsensusSetup {
            tx_sync_interval: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    let (node, mut peer) = handshaken_node_and_peer(setup).await;

    // check if the peer has received a Ping and a GetMemoryPool from the node, in any order
    wait_until!(5, {
        let mut got_ping = false;
        let mut got_getmempool = false;

        loop {
            let payload = peer.read_payload().await;
            match payload {
                Ok(Payload::Ping(..)) => got_ping = true,
                Ok(Payload::GetMemoryPool) => got_getmempool = true,
                _ => {}
            }
            if got_ping && got_getmempool {
                break true;
            }
        }
    });

    // Respond with MemoryPool message
    let memory_pool = Payload::MemoryPool(vec![to_bytes_le![&*TRANSACTION_2].unwrap()]);
    peer.write_message(&memory_pool).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify the transactions have been recevied (and cannot be received again)
    assert!(
        !node
            .expect_sync()
            .consensus
            .receive_transaction(TRANSACTION_2.clone())
            .await
    );
}

#[tokio::test]
async fn transaction_responder_side() {
    // handshake between a fake node and a full node
    let (node, mut peer) = handshaken_node_and_peer(TestSetup::default()).await;

    // check if the peer has received an automatic Ping message from the node
    wait_until!(5, {
        loop {
            let payload = peer.read_payload().await;
            if matches!(payload, Ok(Payload::Ping(..))) {
                break true;
            }
        }
    });

    // insert transactions into node

    assert!(
        node.expect_sync()
            .consensus
            .receive_transaction(TRANSACTION_2.clone())
            .await
    );

    // send a GetMemoryPool message
    let get_memory_pool = Payload::GetMemoryPool;
    peer.write_message(&get_memory_pool).await;

    // check GetMemoryPool message was received
    let payload = peer.read_payload().await.unwrap();
    let txs = if let Payload::MemoryPool(txs) = payload {
        txs
    } else {
        unreachable!();
    };

    // check transactions
    assert!(txs.contains(&to_bytes_le![&*TRANSACTION_2].unwrap()));
}

#[tokio::test]
async fn transaction_two_node() {
    let node_alice = test_node(TestSetup::default()).await;
    let alice_address = node_alice.local_address().unwrap();

    // insert transaction into node_alice
    assert!(
        node_alice
            .expect_sync()
            .consensus
            .receive_transaction(TRANSACTION_2.clone())
            .await
    );

    let setup = TestSetup {
        consensus_setup: Some(ConsensusSetup {
            tx_sync_interval: 1,
            ..Default::default()
        }),
        peer_sync_interval: 1,
        bootnodes: vec![alice_address.to_string()],
        ..Default::default()
    };
    let node_bob = test_node(setup).await;

    // check transaction is present in bob's memory pool
    wait_until!(
        5,
        node_bob
            .expect_sync()
            .consensus
            .fetch_memory_pool()
            .await
            .contains(&TRANSACTION_2)
    );
}
