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

mod consensus_sidechain {
    // use snarkos_storage::validator::FixMode;
    use snarkos_testing::sync::*;

    use rand::{seq::IteratorRandom, thread_rng, Rng};

    // Receive two new blocks out of order.
    // Like the test above, except block 2 is received first as an orphan with no parent.
    // The sync mechanism should push the orphan into storage until block 1 is received.
    // After block 1 is received, block 2 should be fetched from storage and added to the chain.
    #[tokio::test]
    async fn new_out_of_order() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;

        let old_block_height = consensus.storage.canon().await.unwrap().block_height;

        // Find second block

        let block_2 = BLOCK_2.clone();
        assert!(consensus.receive_block(block_2.clone()).await);

        // Find first block

        let block_1 = BLOCK_1.clone();
        assert!(consensus.receive_block(block_1.clone()).await);

        // Check balances after both blocks

        let new_block_height = consensus.storage.canon().await.unwrap().block_height;
        assert_eq!(old_block_height + 2, new_block_height);
    }

    // Receive two blocks that reference the same parent.
    // Treat the first block received as the canonical chain but store and keep the rejected sidechain block in storage.
    #[tokio::test]
    async fn reject() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;

        let block_1_canon = BLOCK_1.clone();
        let block_1_side = ALTERNATIVE_BLOCK_1.clone();

        let old_block_height = consensus.storage.canon().await.unwrap().block_height;

        // 1. Receive canonchain block 1.

        assert!(consensus.receive_block(block_1_canon.clone()).await);

        // 2. Receive sidechain block 1.

        assert!(consensus.receive_block(block_1_side.clone()).await);

        let new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height + 1, new_block_height);

        // 3. Ensure sidechain block 1 rejected.

        let accepted_hash = consensus.storage.canon().await.unwrap().hash;
        let accepted = consensus.storage.get_block(&accepted_hash).await.unwrap();

        assert_ne!(accepted, block_1_side);
    }

    // Receive blocks from a sidechain that overtakes our current canonical chain.
    #[tokio::test]
    async fn accept() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;

        let block_1_canon = ALTERNATIVE_BLOCK_1.clone();
        let block_1_side = BLOCK_1.clone();
        let block_2_side = BLOCK_2.clone();

        // 1. Receive shorter chain of block_1_canon.

        let mut old_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert!(consensus.receive_block(block_1_canon.clone()).await);

        let mut new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height + 1, new_block_height);

        // 2. Receive longer chain of blocks 1 and 2 from the sidechain (the longest chain wins).

        old_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert!(consensus.receive_block(block_1_side.clone()).await);
        assert!(consensus.receive_block(block_2_side.clone()).await);

        new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height + 1, new_block_height);
    }

    // Receive blocks from a sidechain (out of order) that overtakes our current canonical chain.
    #[tokio::test]
    async fn fork_out_of_order() {
        // tracing_subscriber::fmt::init();
        let consensus = snarkos_testing::sync::create_test_consensus().await;

        let block_1_canon = BLOCK_1.clone();
        let block_2_canon = BLOCK_2.clone();
        let block_1_side = ALTERNATIVE_BLOCK_1.clone();
        let block_2_side = ALTERNATIVE_BLOCK_2.clone();

        // 1. Receive irrelevant block.

        let mut old_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert!(consensus.receive_block(block_2_canon.clone()).await);

        let mut new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height, new_block_height);

        // 2. Receive valid sidechain block

        old_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert!(consensus.receive_block(block_1_side.clone()).await);

        new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height + 1, new_block_height);

        // 3. Receive valid canon block 1 and accept the previous irrelevant block 2

        old_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert!(consensus.receive_block(block_1_canon.clone()).await);

        new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height, new_block_height);

        // 4. Receive valid canon block 1 and accept the previous irrelevant block 2

        old_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert!(consensus.receive_block(block_2_side.clone()).await);

        new_block_height = consensus.storage.canon().await.unwrap().block_height;

        assert_eq!(old_block_height + 1, new_block_height);
    }

    #[tokio::test]
    async fn decommit_one_block() {
        let consensus = snarkos_testing::sync::create_test_consensus().await;

        // Introduce one block.
        let block_1 = BLOCK_1.clone();
        assert!(consensus.receive_block(block_1.clone()).await);

        // Verify that the best block number is the same as the block height.
        let mut block_height = consensus.storage.canon().await.unwrap().block_height;
        let mut best_block_number = consensus.storage.canon().await.unwrap().block_height;
        assert_eq!(best_block_number, block_height);

        // Introduce another block.
        let block_2 = BLOCK_2.clone();
        assert!(consensus.receive_block(block_2.clone()).await);

        // Verify that the best block number is the same as the block height.
        block_height = consensus.storage.canon().await.unwrap().block_height;
        best_block_number = consensus.storage.canon().await.unwrap().block_height;
        assert_eq!(best_block_number, block_height);

        // Check if the locator hashes can be found.
        assert!(consensus.storage.get_block_locator_hashes().await.is_ok());

        // Decommit a block.
        let canon_hash = consensus.storage.canon().await.unwrap().hash;
        consensus.force_decommit(canon_hash).await.unwrap();

        // Verify that the best block number is the same as the block height.
        block_height = consensus.storage.canon().await.unwrap().block_height;
        best_block_number = consensus.storage.canon().await.unwrap().block_height;
        assert_eq!(best_block_number, block_height);

        // Check if the locator hashes can still be found.
        assert!(consensus.storage.get_block_locator_hashes().await.is_ok());
    }

    #[tokio::test]
    async fn long_fork_and_sync_no_overlap() {
        // tracing_subscriber::fmt::init();
        let mut rng = thread_rng();

        let consensus1 = snarkos_testing::sync::create_test_consensus().await;
        let consensus2 = snarkos_testing::sync::create_test_consensus().await;

        // Consensus 1 imports a random number of blocks lower than consensus 2.
        let blocks_1 = TestBlocks::load(Some(rng.gen_range(0..=50)), "test_blocks_100_1").0;
        for block in blocks_1 {
            assert!(consensus1.receive_block(block).await);
        }

        // Consensus 2 imports 100 blocks.
        let blocks_2 = TestBlocks::load(Some(100), "test_blocks_100_2").0;
        for block in &blocks_2 {
            assert!(consensus2.receive_block(block.clone()).await);
        }

        // There is no overlap between the 2 instances.
        let consensus1_locator_hashes = consensus1.storage.get_block_locator_hashes().await.unwrap();
        let consensus2_sync_blocks = consensus2
            .storage
            .find_sync_blocks(&consensus1_locator_hashes, 64)
            .await
            .unwrap(); // no common blocks
        assert_eq!(
            &consensus2_sync_blocks[0],
            &consensus2.storage.get_block_hash(1).await.unwrap().unwrap()
        );

        // Consensus 1 imports a few random blocks that consensus 2 has.
        let num_random_blocks = rng.gen_range(1..=50);
        for block in blocks_2.iter().choose_multiple(&mut rng, num_random_blocks) {
            consensus1.receive_block(block.clone()).await; // ignore potential errors (primarily possible duplicates)
        }

        // Consensus 1 imports all the blocks that consensus 2 has, simulating a full sync.
        for block in blocks_2 {
            consensus1.receive_block(block.clone()).await; // ignore potential errors (primarily possible duplicates)
        }

        // The blocks should fully overlap between the 2 instances now.
        let consensus1_locator_hashes = consensus1.storage.get_block_locator_hashes().await.unwrap();
        let consensus2_sync_blocks = consensus2
            .storage
            .find_sync_blocks(&consensus1_locator_hashes, 64)
            .await
            .unwrap();
        assert!(consensus2_sync_blocks.is_empty());

        // Verify the integrity of the block storage for the first instance.
        // todo: assert!(consensus1.ledger.validate(None, FixMode::Nothing).await);
    }

    #[tokio::test]
    async fn long_fork_and_sync_initial_overlap() {
        // tracing_subscriber::fmt::init();
        let mut rng = thread_rng();

        let consensus1 = snarkos_testing::sync::create_test_consensus().await;
        let consensus2 = snarkos_testing::sync::create_test_consensus().await;

        let blocks1 = TestBlocks::load(Some(50), "test_blocks_100_1").0; // side chain blocks
        let blocks2 = TestBlocks::load(Some(100), "test_blocks_100_2").0; // canon blocks

        // Consensus 2 imports 100 blocks.
        for block in &blocks2 {
            assert!(consensus2.receive_block(block.clone()).await);
        }

        // Consensus 1 imports a random number of blocks that consensus 2 has (canon).
        for block in blocks2.iter().take(rng.gen_range(0..=25)) {
            assert!(consensus1.receive_block(block.clone()).await);
        }
        let overlap_height = consensus1.storage.canon().await.unwrap().block_height;

        // There is some initial overlap between the 2 instances.
        let consensus1_locator_hashes = consensus1.storage.get_block_locator_hashes().await.unwrap();
        let consensus2_sync_blocks = consensus2
            .storage
            .find_sync_blocks(&consensus1_locator_hashes, 64)
            .await
            .unwrap(); // no common blocks
        assert_eq!(
            &consensus2_sync_blocks[0],
            &consensus2
                .storage
                .get_block_hash(overlap_height as u32 + 1)
                .await
                .unwrap()
                .unwrap()
        );

        // Consensus 1 imports a random number of side blocks that cause it to fork to the side chain.
        for block in blocks1.iter().take(rng.gen_range(0..=overlap_height as usize + 25)) {
            assert!(consensus1.receive_block(block.clone()).await);
        }

        // Consensus 1 imports a few random blocks that consensus 2 has.
        let num_random_blocks = rng.gen_range(overlap_height..=25) as usize;
        for block in blocks2.iter().choose_multiple(&mut rng, num_random_blocks) {
            let _ = consensus1.receive_block(block.clone()).await; // ignore potential errors (primarily possible duplicates)
        }

        // Consensus 1 imports all the blocks that consensus 2 has, simulating a full sync.
        for block in blocks2 {
            let _ = consensus1.receive_block(block).await; // ignore potential errors (primarily possible duplicates)
        }

        // The blocks should fully overlap between the 2 instances now.
        let consensus1_locator_hashes = consensus1.storage.get_block_locator_hashes().await.unwrap();
        let consensus2_sync_blocks = consensus2
            .storage
            .find_sync_blocks(&consensus1_locator_hashes, 64)
            .await
            .unwrap(); // no common blocks
        assert!(consensus2_sync_blocks.is_empty());

        // Verify the integrity of the block storage for the first instance.
        // todo: assert!(consensus1.ledger.validate(None, FixMode::Nothing).await);
    }

    #[tokio::test]
    async fn forking_back_and_forth() {
        // tracing_subscriber::fmt::init();

        let consensus1 = snarkos_testing::sync::create_test_consensus().await;
        let consensus2 = snarkos_testing::sync::create_test_consensus().await;

        let blocks1 = TestBlocks::load(Some(10), "test_blocks_100_1").0;
        let blocks2 = TestBlocks::load(Some(10), "test_blocks_100_2").0;

        // Consensus 1 imports 10 blocks.
        for block in &blocks1 {
            assert!(consensus1.receive_block(block.clone()).await);
        }

        // Consensus 2 imports a side chain block and a canon block one after the other.
        for i in 0..10 {
            if i % 2 == 0 {
                consensus2.receive_block(blocks1[i].clone()).await;
                consensus2.receive_block(blocks1[i + 1].clone()).await;
            } else {
                if i == 1 {
                    consensus2.receive_block(blocks2[i - 1].clone()).await;
                }
                consensus2.receive_block(blocks2[i].clone()).await;
                if i != 9 {
                    consensus2.receive_block(blocks2[i + 1].clone()).await;
                }
            }
        }

        // The blocks should fully overlap between the 2 instances now.
        let consensus1_locator_hashes = consensus1.storage.get_block_locator_hashes().await.unwrap();
        let sync_blocks = consensus2
            .storage
            .find_sync_blocks(&consensus1_locator_hashes, 64)
            .await
            .unwrap();
        assert!(sync_blocks.is_empty());

        // todo: assert!(consensus2.ledger.validate(None, FixMode::Nothing).await);
    }

    #[tokio::test]
    async fn decommit_many_and_reimport() {
        // tracing_subscriber::fmt::init();

        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let blocks = TestBlocks::load(Some(20), "test_blocks_100_1").0;

        // Consensus imports 20 blocks.
        for block in &blocks {
            assert!(consensus.receive_block(block.clone()).await);
        }

        // Consensus decommits 10 blocks.
        consensus.force_decommit(blocks[10].header.hash()).await.unwrap();

        assert_eq!(consensus.storage.canon().await.unwrap().block_height, 10);

        consensus.fast_forward().await.unwrap();

        assert_eq!(consensus.storage.canon().await.unwrap().block_height, 20);

        // Verify the integrity of the block storage.
        // todo: assert!(consensus.ledger.validate(None, FixMode::Nothing).await);
    }
}
