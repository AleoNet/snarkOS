mod consensus_receive_block {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_objects::block::Block;
    use snarkos_storage::test_data::*;

    // Receive two new blocks in order.
    // Block 1 references the genesis block and block 2 references block 1
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

    // Receive two new blocks out of order.
    // Like the test above, except block 2 is received first as an orphan with no parent.
    // The consensus mechanism should push the orphan into storage until block 1 is received.
    // After block 1 is received, block 2 should be fetched from storage and added to the chain.
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

    // Receive two blocks with identical hashes.
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
