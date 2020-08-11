mod consensus_integration {
    use snarkos_consensus::miner::Miner;
    use snarkos_dpc::base_dpc::instantiated::Tx;
    use snarkos_objects::{dpc::DPCTransactions, BlockHeader};
    use snarkos_posw::txids_to_roots;
    use snarkos_testing::consensus::*;

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    fn test_find_block(transactions: &DPCTransactions<Tx>, parent_header: &BlockHeader) {
        let consensus = TEST_CONSENSUS.clone();
        let miner_address = FIXTURE_VK.test_accounts[0].address.clone();
        let miner = Miner::new(miner_address, consensus.clone());

        let header = miner.find_block(transactions, parent_header).unwrap();

        let expected_prev_block_hash = parent_header.get_hash();
        assert_eq!(header.previous_block_hash, expected_prev_block_hash);

        let expected_merkle_root_hash = snarkos_objects::merkle_root(&transactions.to_transaction_ids().unwrap());
        assert_eq!(&header.merkle_root_hash.0[..], &expected_merkle_root_hash[..]);

        // generate the verifier args
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transactions.to_transaction_ids().unwrap());

        // ensure that our POSW proof passes
        consensus
            .verify_header(&header, parent_header, &merkle_root, &pedersen_merkle_root)
            .unwrap();
    }

    #[test]
    fn find_valid_block() {
        let transactions = DPCTransactions(vec![
            DATA.block_1.transactions.0[0].clone(),
            DATA.block_2.transactions.0[0].clone(),
        ]);
        let parent_header = genesis().header;
        test_find_block(&transactions, &parent_header);
    }
}
