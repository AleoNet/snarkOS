mod consensus_receive_single_blocks {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_objects::Block;

    // Notes:
    //  - "duplicate block" refers to an identical block with the same block hash
    //  - "already mined block" refers to a block with a different block hash but the same previous_block_hash

    #[test]
    fn new_in_order() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        // Find first block

        let block_1 = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        // Check balances after 1st block

        check_block_1_balances(&blockchain);

        // Find second block

        let block_2 = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2)
            .unwrap();

        // Check balances after the 2nd block

        check_block_2_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn new_out_of_order() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        // Find second block

        let block_2 = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2)
            .unwrap();

        // Find first block

        let block_1 = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        // Check balances after both blocks

        check_block_2_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn duplicate() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1 = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();

        // Receive block 1 twice

        let old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1.clone())
            .unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        // Ensure only 1 block was added

        assert_eq!(old_block_height + 1, new_block_height);

        // Check balances after 1st block

        check_block_1_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }
}
