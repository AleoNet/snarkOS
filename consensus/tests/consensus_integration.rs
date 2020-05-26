mod consensus_integration {
    use snarkos_consensus::{miner::Miner, test_data::*};
    use snarkos_dpc::dpc::base_dpc::instantiated::{Components, Tx};
    use snarkos_models::parameters::Parameter;
    use snarkos_objects::{dpc::DPCTransactions, AccountPublicKey, BlockHeader};
    use snarkos_parameters::genesis_account::GenesisAccount;
    use snarkos_utilities::bytes::FromBytes;

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    fn test_find_block(transactions: &DPCTransactions<Tx>, parent_header: &BlockHeader) {
        let consensus = TEST_CONSENSUS;
        let miner_address: AccountPublicKey<Components> = FromBytes::read(&GenesisAccount::load_bytes()[..]).unwrap();
        let miner = Miner::new(miner_address, consensus);

        let header = miner.find_block(transactions, parent_header).unwrap();

        let expected_prev_block_hash = parent_header.get_hash();
        assert_eq!(header.previous_block_hash, expected_prev_block_hash);

        let expected_merkle_root_hash = snarkos_objects::merkle_root(&transactions.to_transaction_ids().unwrap());
        assert_eq!(&header.merkle_root_hash.0[..], &expected_merkle_root_hash[..]);
    }

    #[test]
    fn find_valid_block() {
        let transactions = DPCTransactions(vec![
            DATA.block1.transactions.0[0].clone(),
            DATA.block2.transactions.0[0].clone(),
        ]);
        let parent_header = genesis().header;
        test_find_block(&transactions, &parent_header);
    }
}
