mod consensus_blocks {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_dpc::base_dpc::instantiated::Tx;
    use snarkos_objects::Block;

    // Receive two new blocks in order.
    #[test]
    fn new_in_order() {
        let (mut blockchain, path) = initialize_test_blockchain();

        let parameters = load_verifying_parameters();

        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let old_block_height = blockchain.get_latest_block_height();

        // Find first block

        let block_1 = Block::<Tx>::deserialize(&BLOCK_1.to_vec()).unwrap();
        consensus
            .receive_block(&parameters, &mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        // Duplicate blocks dont do anything
        consensus
            .receive_block(&parameters, &mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();
        assert_eq!(old_block_height + 1, new_block_height);

        // Find second block

        let block_2 = Block::<Tx>::deserialize(&BLOCK_2.to_vec()).unwrap();
        consensus
            .receive_block(&parameters, &mut blockchain, &mut memory_pool, &block_2)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();
        assert_eq!(old_block_height + 2, new_block_height);

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn remove_latest_block() {
        let (mut blockchain, path) = initialize_test_blockchain();

        let parameters = load_verifying_parameters();

        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let old_block_height = blockchain.get_latest_block_height();

        // Find first block

        let block_1 = Block::<Tx>::deserialize(&BLOCK_1.to_vec()).unwrap();
        consensus
            .receive_block(&parameters, &mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        blockchain.remove_latest_blocks(1).unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height, new_block_height);

        kill_storage_sync(blockchain, path);
    }

    // TODO Implement Orphan block handling

    //    // Receive two new blocks out of order.
    //    // Like the test above, except block 2 is received first as an orphan with no parent.
    //    // The consensus mechanism should push the orphan into storage until block 1 is received.
    //    // After block 1 is received, block 2 should be fetched from storage and added to the chain.
    //    #[test]
    //    fn new_out_of_order() {
    //        let (mut blockchain, path) = initialize_test_blockchain();
    //        let mut memory_pool = MemoryPool::new();
    //
    //        let consensus = TEST_CONSENSUS;
    //
    //        // Find second block
    //
    //        let block_2 = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();
    //        consensus
    //            .receive_block(&mut blockchain, &mut memory_pool, &block_2)
    //            .unwrap();
    //
    //        // Find first block
    //
    //        let block_1 = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
    //        consensus
    //            .receive_block(&mut blockchain, &mut memory_pool, &block_1)
    //            .unwrap();
    //
    //        // Check balances after both blocks
    //
    //        check_block_2_balances(&blockchain);
    //
    //        kill_storage_sync(blockchain, path);
    //    }
}
