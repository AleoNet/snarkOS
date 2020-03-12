mod consensus_sidechain {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_objects::Block;
    use snarkos_storage::test_data::*;

    // Receive two blocks that reference the same parent.
    // Treat the first block received as the canonical chain but store and keep the rejected sidechain block in storage.
    #[test]
    fn reject() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1_canon = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        let block_1_side = Block::deserialize(&hex::decode(&BLOCK_1_LATE).unwrap()).unwrap();

        let old_block_height = blockchain.get_latest_block_height();

        // 1. Receive canonchain block 1.

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_canon)
            .unwrap();

        // 2. Receive sidechain block 1.

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_side.clone())
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        // 3. Ensure sidechain block 1 rejected.

        let accepted = blockchain.get_latest_block().unwrap();

        assert_ne!(accepted, block_1_side);

        // 4. Check balances after block 1.

        check_block_1_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    // Receive blocks from a sidechain that overtakes our current canonical chain.
    #[test]
    fn accept() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1_canon = Block::deserialize(&hex::decode(&BLOCK_1_LATE).unwrap()).unwrap();
        let block_1_side = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        let block_2_side = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();

        // 1. Receive shorter chain of block_1_canon.

        let mut old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_canon)
            .unwrap();

        let mut new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        // 2. Receive longer chain of blocks 1 and 2 from the sidechain (the longest chain wins).

        old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_side)
            .unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2_side)
            .unwrap();

        new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        // 3. Ensure the two sidechain blocks were accepted and the first block_1_canon was overwritten.

        check_block_2_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    // Receive 5 blocks in total.
    // 1. Receive block_1_canon
    // 2. Receive block_1_side + block_2_side overwriting 1.
    // 3. Receive block_2_canon + block_3_canon overwriting 2. and restoring 1.
    #[test]
    fn accept_then_reject() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1_canon = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        let block_2_canon = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();
        let block_3_canon = Block::deserialize(&hex::decode(&BLOCK_3).unwrap()).unwrap();

        let block_1_side = Block::deserialize(&hex::decode(&BLOCK_1_LATE).unwrap()).unwrap();
        let block_2_side = Block::deserialize(&hex::decode(&BLOCK_2_LATE).unwrap()).unwrap();

        // 1. Receive block_1_canon.

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_canon)
            .unwrap();

        // 2. Receive block_1_side, block_2_side switching to side chain.

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_side)
            .unwrap();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2_side)
            .unwrap();

        // 3. Receive block_2_canon, block_3_canon switching back to canonical chain.

        let old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2_canon)
            .unwrap();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_3_canon)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        check_block_3_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }
}
