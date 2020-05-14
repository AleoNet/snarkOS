mod consensus_dpc {
    use snarkos_consensus::{
        get_block_reward,
        miner::{MemoryPool, Miner},
        test_data::*,
        ConsensusParameters,
    };
    use snarkos_dpc::{
        base_dpc::{instantiated::*, record::DPCRecord, record_payload::PaymentRecordPayload, BaseDPCComponents},
        test_data::*,
        DPCScheme,
    };
    use snarkos_models::dpc::Record;
    use snarkos_objects::{
        dpc::{Block, DPCTransactions},
        ledger::Ledger,
    };
    use snarkos_storage::BlockStorage;
    use snarkos_utilities::{bytes::ToBytes, to_bytes};

    use rand::{thread_rng, Rng};

    #[test]
    fn base_dpc_multiple_transactions() {
        let mut rng = thread_rng();

        let consensus = TEST_CONSENSUS.clone();
        let mut memory_pool = MemoryPool::new();

        // Generate or load parameters for the ledger, commitment schemes, and CRH
        let (ledger_parameters, parameters) = setup_or_load_parameters(false, &mut rng);

        // Generate addresses
        let [genesis_address, miner_address, recipient] = generate_test_accounts(&parameters, &mut rng);

        let mut path = std::env::temp_dir();
        let random_storage_path: usize = rng.gen();
        path.push(format!("test_multiple_transations_db{}", random_storage_path));

        let (ledger, genesis_pred_vk_bytes) =
            setup_ledger(&path, &parameters, ledger_parameters, &genesis_address, &mut rng);
        let miner = Miner::new(miner_address.public_key, consensus.clone(), POSW_PP.clone().0);

        // Initialize the predicate values
        let new_predicate = Predicate::new(genesis_pred_vk_bytes);
        let new_birth_predicates = vec![new_predicate.clone(); NUM_INPUT_RECORDS];
        let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];

        let transactions = DPCTransactions::<Tx>::new();

        println!("Creating block with coinbase transaction");

        let (previous_block_header, transactions, coinbase_records) =
            miner.establish_block(&parameters, &ledger, &transactions).unwrap();

        let header = miner.find_block(&transactions, &previous_block_header, &mut rng).unwrap();

        let block = Block { header, transactions };

        assert!(InstantiatedDPC::verify(&parameters, &block.transactions[0], &ledger).unwrap());

        let block_reward = get_block_reward(ledger.len() as u32);

        assert_eq!(coinbase_records.len(), 2);
        assert!(!coinbase_records[0].is_dummy());
        assert!(coinbase_records[1].is_dummy());
        assert_eq!(coinbase_records[0].payload().balance, block_reward);
        assert_eq!(coinbase_records[1].payload().balance, 0);

        println!("Verifying and receiving the block");
        consensus
            .receive_block(&parameters, &ledger, &mut memory_pool, &block)
            .unwrap();

        assert_eq!(ledger.len(), 2);

        // Add new block spending records from the previous block

        let old_account_private_keys = vec![miner_address.private_key.clone(); NUM_INPUT_RECORDS];
        let new_account_public_keys = vec![recipient.public_key.clone(); NUM_OUTPUT_RECORDS];

        let new_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];
        let new_payload = PaymentRecordPayload { balance: 10, lock: 0 };

        let new_payloads = vec![new_payload.clone(); NUM_OUTPUT_RECORDS];

        let auxiliary = [5u8; 32];
        let memo = [6u8; 32];

        let mut transactions = DPCTransactions::new();

        println!("Create a payment transaction transaction");

        let (spend_records, transaction) = ConsensusParameters::create_transaction(
            &parameters,
            coinbase_records,
            old_account_private_keys,
            new_account_public_keys,
            new_birth_predicates.clone(),
            new_death_predicates.clone(),
            new_dummy_flags,
            new_payloads,
            auxiliary,
            memo,
            &ledger,
            &mut rng,
        )
        .unwrap();

        assert_eq!(spend_records.len(), 2);
        assert!(!spend_records[0].is_dummy());
        assert!(!spend_records[1].is_dummy());
        assert_eq!(spend_records[0].payload().balance, 10);
        assert_eq!(spend_records[1].payload().balance, 10);
        assert_eq!(transaction.stuff.value_balance, (block_reward - 20) as i64);

        assert!(InstantiatedDPC::verify(&parameters, &transaction, &ledger).unwrap());

        transactions.push(transaction);

        println!("Create a new block with the payment transaction");

        let (previous_block_header, transactions, new_coinbase_records) =
            miner.establish_block(&parameters, &ledger, &transactions).unwrap();

        let header = miner.find_block(&transactions, &previous_block_header, &mut rand::thread_rng()).unwrap();

        let new_block = Block { header, transactions };

        let new_block_reward = get_block_reward(ledger.len() as u32);

        assert_eq!(new_coinbase_records.len(), 2);
        assert!(!new_coinbase_records[0].is_dummy());
        assert!(new_coinbase_records[1].is_dummy());
        assert_eq!(
            new_coinbase_records[0].payload().balance,
            new_block_reward + block_reward - 20
        );
        assert_eq!(new_coinbase_records[1].payload().balance, 0);

        println!("Verify and receive the block with the new payment transaction");

        consensus
            .receive_block(&parameters, &ledger, &mut memory_pool, &new_block)
            .unwrap();

        assert_eq!(ledger.len(), 3);

        for record in &new_coinbase_records {
            ledger.store_record(record).unwrap();

            let reconstruct_record: Option<DPCRecord<Components>> = ledger
                .get_record(&to_bytes![record.commitment()].unwrap().to_vec())
                .unwrap();

            assert_eq!(
                to_bytes![reconstruct_record.unwrap()].unwrap(),
                to_bytes![record].unwrap()
            );
        }

        let path = ledger.storage.storage.path().to_owned();
        drop(ledger);
        BlockStorage::<Tx, <Components as BaseDPCComponents>::MerkleParameters>::destroy_storage(path).unwrap();
    }
}
