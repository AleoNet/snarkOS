mod miner {
    use snarkos_consensus::Miner;
    use snarkos_models::{
        algorithms::{commitment::CommitmentScheme, signature::SignatureScheme},
        dpc::DPCComponents,
    };
    use snarkos_objects::{dpc::DPCTransactions, AccountPrivateKey, AccountPublicKey, BlockHeader};
    use snarkos_posw::txids_to_roots;
    use snarkos_testing::consensus::*;

    use rand::{Rng, SeedableRng};
    use rand_xorshift::XorShiftRng;

    fn keygen<C: DPCComponents, R: Rng>(rng: &mut R) -> (AccountPrivateKey<C>, AccountPublicKey<C>) {
        let sig_params = C::AccountSignature::setup(rng).unwrap();
        let comm_params = C::AccountCommitment::setup(rng);

        let key = AccountPrivateKey::<C>::new(&sig_params, &comm_params, &[0; 32], rng).unwrap();
        let pubkey = AccountPublicKey::from(key.commitment(&sig_params, &comm_params).unwrap()).unwrap();

        (key, pubkey)
    }

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    fn test_find_block(transactions: &DPCTransactions<TestTx>, parent_header: &BlockHeader) {
        let consensus = TEST_CONSENSUS.clone();
        let mut rng = XorShiftRng::seed_from_u64(3); // use this rng so that a valid solution is found quickly

        let (_, miner_address) = keygen(&mut rng);
        let miner = Miner::new(miner_address, consensus.clone());

        let header = miner.find_block(transactions, parent_header).unwrap();

        // generate the verifier args
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transactions.to_transaction_ids().unwrap());

        // ensure that our POSW proof passes
        consensus
            .verify_header(&header, parent_header, &merkle_root, &pedersen_merkle_root)
            .unwrap();
    }

    #[test]
    fn find_valid_block() {
        let transactions = DPCTransactions(vec![TestTx; 3]);
        let parent_header = genesis().header;
        test_find_block(&transactions, &parent_header);
    }
}
