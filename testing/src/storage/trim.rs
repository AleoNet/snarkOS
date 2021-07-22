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

use crate::sync::{create_test_consensus, TestBlocks};
use snarkos_storage::*;

use std::collections::HashSet;

#[tokio::test]
async fn trim_side_chain_blocks() {
    let consensus = create_test_consensus();

    // Register the hash of the genesis block.
    let genesis_hash = consensus.storage.get_block_hash(0).await.unwrap().unwrap();
    let genesis_block = consensus.storage.get_block(&genesis_hash).await.unwrap();

    // Import a few consecutive blocks from one chain.
    let blocks1 = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    let mut block_hashes1 = HashSet::new();
    for block in &blocks1 {
        let hash = block.header.hash();
        block_hashes1.insert(hash);

        assert!(consensus.receive_block(block.clone()).await);
    }

    assert_eq!(
        consensus.storage.longest_child_path(&genesis_hash).await.unwrap().len(),
        6
    );

    // Import a second, longer chain.
    let blocks2 = TestBlocks::load(Some(10), "test_blocks_100_2").0;
    let mut block_hashes2 = HashSet::new();
    for block in blocks2 {
        let hash = block.header.hash();
        block_hashes2.insert(hash);

        assert!(consensus.receive_block(block).await);
    }

    assert_eq!(
        consensus.storage.longest_child_path(&genesis_hash).await.unwrap().len(),
        11
    );

    // Ensure that the obsolete objects are in place.
    for block in &blocks1 {
        // Check the header.
        let hash = block.header.hash();
        assert_eq!(
            consensus.storage.get_block_state(&hash).await.unwrap(),
            BlockStatus::Uncommitted
        );

        // Check the txs.
        consensus.storage.get_block(&hash).await.unwrap();

        // Check tx locations.
        for tx in &block.transactions {
            let tx_location = consensus
                .storage
                .get_transaction_location(tx.id.into())
                .await
                .unwrap()
                .unwrap();
            assert!(block_hashes1.contains(&tx_location.block_hash));
        }
    }

    // Trim the storage; only the objects related to canon blocks should remain now.
    trim(consensus.storage.clone()).await.unwrap();

    // Ensure that the obsolete objects were trimmed.
    for block in blocks1 {
        // Check the header.
        let hash = block.header.hash();
        assert_eq!(
            consensus.storage.get_block_state(&hash).await.unwrap(),
            BlockStatus::Unknown
        );

        // Check the txs.
        assert!(consensus.storage.get_block(&hash).await.is_err());

        // Check tx locations.
        for tx in &block.transactions {
            // A transaction from the first set of blocks could still be applicable...
            let tx_location = match consensus.storage.get_transaction_location(tx.id.into()).await.unwrap() {
                Some(location) => location,
                None => continue,
            };

            // ...but if it is, it must exist in the second set of blocks too.
            assert!(block_hashes2.contains(&tx_location.block_hash));
        }
    }

    // Ensure that children of the genesis block were cleaned up.
    //todo: assert_eq!(consensus.ledger.get_child_block_hashes(&genesis_hash).unwrap().len(), 1);

    // Validate the post-trim storage.
    // todo: assert!(consensus.ledger.validate(None, FixMode::Nothing).await);
}
