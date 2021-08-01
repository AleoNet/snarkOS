// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

mod consensus_dpc {
    use snarkos_consensus::{get_block_reward, Miner};
    use snarkos_testing::sync::*;
    use snarkvm::{
        dpc::{payload::Payload as RecordPayload, record::Record, testnet1::*, DPCScheme, Parameters, RecordScheme},
        ledger::prelude::*,
        utilities::{to_bytes_le, ToBytes},
    };

    use std::sync::Arc;

    #[tokio::test]
    async fn base_dpc_multiple_transactions() {
        let program = FIXTURE.program.clone();
        let [_genesis_address, miner_acc, recipient] = FIXTURE.test_accounts.clone();
        let mut rng = FIXTURE.rng.clone();

        let consensus = Arc::new(snarkos_testing::sync::create_test_consensus());
        let miner = Miner::new(miner_acc.address, consensus.clone());

        println!("Creating block with coinbase transaction");
        let transactions = Transactions::<Testnet1Transaction>::new();
        let (previous_block_header, transactions, coinbase_records) = miner.establish_block(&transactions).unwrap();
        let header = miner.find_block(&transactions, &previous_block_header).unwrap();
        let block = Block { header, transactions };

        assert!(Testnet1DPC::verify_transactions(
            &consensus.dpc,
            &block.transactions,
            &*consensus.ledger
        ));

        let block_reward = get_block_reward(consensus.ledger.block_height() as u32);

        // dummy outputs have 0 balance, coinbase only pays the miner
        assert_eq!(coinbase_records.len(), 2);
        assert!(!coinbase_records[0].is_dummy());
        assert!(coinbase_records[1].is_dummy());
        assert_eq!(coinbase_records[0].value(), block_reward.0 as u64);
        assert_eq!(coinbase_records[1].value(), 0);

        println!("Verifying and receiving the block");
        consensus.receive_block(&block, false).await.unwrap();
        assert_eq!(consensus.ledger.block_height(), 2);

        // Add new block spending records from the previous block

        // INPUTS

        let old_account_private_keys = vec![miner_acc.private_key; Testnet1Parameters::NUM_INPUT_RECORDS];
        let old_records = coinbase_records;

        // OUTPUTS

        let mut joint_serial_numbers = vec![];
        for i in 0..Testnet1Parameters::NUM_INPUT_RECORDS {
            let (sn, _) = old_records[i].to_serial_number(&old_account_private_keys[i]).unwrap();
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn].unwrap());
        }

        let mut new_records = vec![];
        for j in 0..Testnet1Parameters::NUM_OUTPUT_RECORDS {
            new_records.push(
                Record::new_full(
                    &program,
                    recipient.address.clone(),
                    false,
                    10,
                    RecordPayload::default(),
                    (Testnet1Parameters::NUM_INPUT_RECORDS + j) as u8,
                    joint_serial_numbers.clone(),
                    &mut rng,
                )
                .unwrap(),
            );
        }

        let spend_records = new_records.clone();

        println!("Create a payment transaction");
        // Create the transaction
        let transaction = consensus
            .create_transaction(old_records, old_account_private_keys, new_records, None, &mut rng)
            .unwrap();

        assert_eq!(spend_records.len(), 2);
        assert!(!spend_records[0].is_dummy());
        assert!(!spend_records[1].is_dummy());
        assert_eq!(spend_records[0].value(), 10);
        assert_eq!(spend_records[1].value(), 10);
        assert_eq!(transaction.value_balance.0, (block_reward.0 - 20) as i64);

        assert!(Testnet1DPC::verify(&consensus.dpc, &transaction, &*consensus.ledger));

        println!("Create a new block with the payment transaction");
        let mut transactions = Transactions::new();
        transactions.push(transaction);
        let (previous_block_header, transactions, new_coinbase_records) = miner.establish_block(&transactions).unwrap();

        assert!(Testnet1DPC::verify_transactions(
            &consensus.dpc,
            &transactions,
            &*consensus.ledger
        ));

        let header = miner.find_block(&transactions, &previous_block_header).unwrap();
        let new_block = Block { header, transactions };
        let new_block_reward = get_block_reward(consensus.ledger.block_height() as u32);

        assert_eq!(new_coinbase_records.len(), 2);
        assert!(!new_coinbase_records[0].is_dummy());
        assert!(new_coinbase_records[1].is_dummy());
        assert_eq!(
            new_coinbase_records[0].value(),
            (new_block_reward.0 + block_reward.0 - 20) as u64
        );
        assert_eq!(new_coinbase_records[1].value(), 0);

        println!("Verify and receive the block with the new payment transaction");

        consensus.receive_block(&new_block, false).await.unwrap();

        assert_eq!(consensus.ledger.block_height(), 3);

        for record in &new_coinbase_records {
            consensus.ledger.store_record(record).unwrap();

            let reconstruct_record: Option<Record<Testnet1Parameters>> = consensus
                .ledger
                .get_record(&to_bytes_le![record.commitment()].unwrap().to_vec())
                .unwrap();

            assert_eq!(
                to_bytes_le![reconstruct_record.unwrap()].unwrap(),
                to_bytes_le![record].unwrap()
            );
        }
    }
}
