mod consensus_sidechain {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_objects::Block;

    #[test]
    fn reject() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1_main = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        let block_1_side = Block::deserialize(&hex::decode(&BLOCK_1_LATE).unwrap()).unwrap();

        let old_block_height = blockchain.get_latest_block_height();

        // 1. Receive earlier block 1

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_main)
            .unwrap();

        // 2. Receive sidechain block 1

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_side.clone())
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        // 3. Ensure sidechain block 1 rejected

        let accepted = blockchain.get_latest_block().unwrap();

        assert_ne!(accepted, block_1_side);

        // 4. Check balances after block 1

        check_block_1_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn accept() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1_main = Block::deserialize(&hex::decode(&BLOCK_1_LATE).unwrap()).unwrap();
        let block_1_side = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        let block_2_side = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();

        // 1. Receive shorter chain of block_1_mainchain

        let mut old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_main)
            .unwrap();

        let mut new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        // 2. Receive longer chain of blocks 1 and 2 from the sidechain (the longest chain wins)

        old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_side)
            .unwrap();
        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2_side)
            .unwrap();

        new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        check_block_2_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn accept_then_reject() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;

        let block_1_main = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
        let block_2_main = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();
        let block_3_main = Block::deserialize(&hex::decode(&BLOCK_3).unwrap()).unwrap();

        let block_1_side = Block::deserialize(&hex::decode(&BLOCK_1_LATE).unwrap()).unwrap();
        let block_2_side = Block::deserialize(&hex::decode(&BLOCK_2_LATE).unwrap()).unwrap();

        // 1. Receive b1M

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_main)
            .unwrap();

        // 2. Receive b1S b2S, switching to side chain S

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_1_side)
            .unwrap();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2_side)
            .unwrap();

        // 3. Receive b2M, b3M, switching back to main chain M

        let old_block_height = blockchain.get_latest_block_height();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_2_main)
            .unwrap();

        consensus
            .receive_block(&mut blockchain, &mut memory_pool, &block_3_main)
            .unwrap();

        let new_block_height = blockchain.get_latest_block_height();

        assert_eq!(old_block_height + 1, new_block_height);

        check_block_3_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }
}
