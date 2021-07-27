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
use snarkvm_dpc::{BlockHeaderHash, LedgerScheme};

use std::collections::HashSet;

#[tokio::test]
async fn trim_side_chain_blocks() {
    let consensus = create_test_consensus();

    // Register the hash of the genesis block.
    let genesis_block = consensus.ledger.get_block_from_block_number(0).unwrap();
    let genesis_hash = genesis_block.header.get_hash();

    // Import a few consecutive blocks from one chain.
    let blocks1 = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    let mut block_hashes1 = HashSet::new();
    for block in &blocks1 {
        let hash = block.header.get_hash();
        block_hashes1.insert(hash);

        consensus.receive_block(&block, false).await.unwrap();
    }

    assert_eq!(consensus.ledger.get_child_block_hashes(&genesis_hash).unwrap().len(), 1);

    // Import a second, longer chain.
    let blocks2 = TestBlocks::load(Some(10), "test_blocks_100_2").0;
    let mut block_hashes2 = HashSet::new();
    for block in blocks2 {
        let hash = block.header.get_hash();
        block_hashes2.insert(hash);

        consensus.receive_block(&block, false).await.unwrap();
    }

    assert_eq!(consensus.ledger.get_child_block_hashes(&genesis_hash).unwrap().len(), 2);

    // Ensure that the obsolete objects are in place.
    for block in &blocks1 {
        // Check the header.
        let hash = block.header.get_hash();
        assert!(consensus.ledger.contains_block_hash(&hash));

        // Check the txs.
        assert!(consensus.ledger.get_block_transactions(&hash).is_ok());

        // Check tx locations.
        for tx_id in block.transactions.to_transaction_ids().unwrap() {
            let tx_location = consensus.ledger.get_transaction_location(&tx_id).unwrap().unwrap();
            let tx_block_hash = BlockHeaderHash::new(tx_location.block_hash.to_vec());
            assert!(block_hashes1.contains(&tx_block_hash));
        }
    }

    // Trim the storage; only the objects related to canon blocks should remain now.
    consensus.ledger.trim().unwrap();

    // Ensure that the obsolete objects were trimmed.
    for block in blocks1 {
        // Check the header.
        let hash = block.header.get_hash();
        assert!(!consensus.ledger.contains_block_hash(&hash));

        // Check the txs.
        assert!(consensus.ledger.get_block_transactions(&hash).is_err());

        // Check tx locations.
        for tx_id in block.transactions.to_transaction_ids().unwrap() {
            // A transaction from the first set of blocks could still be applicable...
            let tx_location = match consensus.ledger.get_transaction_location(&tx_id).unwrap() {
                Some(location) => BlockHeaderHash::new(location.block_hash.to_vec()),
                None => continue,
            };

            // ...but if it is, it must exist in the second set of blocks too.
            assert!(block_hashes2.contains(&tx_location));
        }
    }

    // Ensure that children of the genesis block were cleaned up.
    assert_eq!(consensus.ledger.get_child_block_hashes(&genesis_hash).unwrap().len(), 1);

    // Validate the post-trim storage.
    assert!(consensus.ledger.validate(None, FixMode::Nothing).await);
}
