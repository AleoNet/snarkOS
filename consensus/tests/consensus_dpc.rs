mod consensus_dpc {
    use snarkos_consensus::{
        get_block_reward,
        miner::{MemoryPool, Miner},
        test_data::*,
        ConsensusParameters,
        GM17Verifier,
    };
    use snarkos_dpc::base_dpc::{instantiated::*, record::DPCRecord, record_payload::PaymentRecordPayload};
    use snarkos_models::{
        dpc::{DPCScheme, Record},
        objects::LedgerScheme,
    };
    use snarkos_objects::{dpc::DPCTransactions, Block};
    use snarkos_storage::test_data::kill_storage;
    use snarkos_utilities::{bytes::ToBytes, to_bytes};

    #[test]
    fn base_dpc_multiple_transactions() {
        let parameters = &FIXTURE.parameters;
        let ledger = FIXTURE.ledger();
        let predicate = FIXTURE.predicate.clone();
        let [_genesis_address, miner_acc, recipient] = FIXTURE.test_accounts.clone();
        let mut rng = FIXTURE.rng.clone();

        let consensus = TEST_CONSENSUS.clone();
        let miner = Miner::new(miner_acc.public_key, consensus.clone(), POSW_PP.0.clone());

        println!("Creating block with coinbase transaction");
        let transactions = DPCTransactions::<Tx>::new();
        let (previous_block_header, transactions, coinbase_records) =
            miner.establish_block(&parameters, &ledger, &transactions).unwrap();
        let header = miner
            .find_block(&transactions, &previous_block_header, &mut rng)
            .unwrap();
        let block = Block { header, transactions };

        assert!(InstantiatedDPC::verify_transactions(&parameters, &block.transactions, &ledger).unwrap());

        let block_reward = get_block_reward(ledger.len() as u32);

        // dummy outputs have 0 balance, coinbase only pays the miner
        assert_eq!(coinbase_records.len(), 2);
        assert!(!coinbase_records[0].is_dummy());
        assert!(coinbase_records[1].is_dummy());
        assert_eq!(coinbase_records[0].payload().balance, block_reward);
        assert_eq!(coinbase_records[1].payload().balance, 0);

        println!("Verifying and receiving the block");
        let mut memory_pool = MemoryPool::new();
        consensus
            .receive_block(&parameters, &ledger, &mut memory_pool, &block)
            .unwrap();
        assert_eq!(ledger.len(), 2);

        // Add new block spending records from the previous block

        // INPUTS

        let old_account_private_keys = vec![miner_acc.private_key; NUM_INPUT_RECORDS];
        let old_records = coinbase_records;
        let new_birth_predicates = vec![predicate.clone(); NUM_INPUT_RECORDS];

        // OUTPUTS

        let new_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];
        let new_account_public_keys = vec![recipient.public_key.clone(); NUM_OUTPUT_RECORDS];
        let new_death_predicates = vec![predicate; NUM_OUTPUT_RECORDS];
        let new_payloads = vec![PaymentRecordPayload { balance: 10, lock: 0 }; NUM_OUTPUT_RECORDS];

        // Memo + Aux are dummies for now

        let auxiliary = [5u8; 32];
        let memo = [6u8; 32];

        println!("Create a payment transaction");
        // Create the transaction
        let (spend_records, transaction) = ConsensusParameters::<GM17Verifier>::create_transaction(
            &parameters,
            old_records,
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

        println!("Create a new block with the payment transaction");
        let mut transactions = DPCTransactions::new();
        transactions.push(transaction);
        let (previous_block_header, transactions, new_coinbase_records) =
            miner.establish_block(&parameters, &ledger, &transactions).unwrap();

        assert!(InstantiatedDPC::verify_transactions(&parameters, &transactions, &ledger).unwrap());

        let header = miner
            .find_block(&transactions, &previous_block_header, &mut rng)
            .unwrap();
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

        kill_storage(ledger);
    }
}
