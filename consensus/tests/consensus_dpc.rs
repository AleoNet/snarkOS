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
    use rand::thread_rng;
    use snarkos_consensus::{get_block_reward, CreateTransactionRequest, MineContext};
    use snarkos_storage::{SerialBlock, VMRecord};
    use snarkos_testing::sync::*;
    use snarkvm_dpc::{
        testnet1::{instantiated::*, payload::Payload as RecordPayload, record::Record as DPCRecord},
        DPCComponents,
        ProgramScheme,
        RecordScheme,
    };
    use snarkvm_utilities::{to_bytes_le, ToBytes};

    #[tokio::test]
    async fn base_dpc_multiple_transactions() {
        let program = FIXTURE.program.clone();
        let [_genesis_address, miner_acc, recipient] = FIXTURE.test_accounts.clone();

        let consensus = snarkos_testing::sync::create_test_consensus().await;

        let miner = MineContext::prepare(miner_acc.address, consensus.clone())
            .await
            .unwrap();

        println!("Creating block with coinbase transaction");
        let (transactions, coinbase_records) = miner.establish_block(vec![]).await.unwrap();
        let previous_block_header = genesis().header.into();
        let header = miner.find_block(&transactions, &previous_block_header).unwrap();
        let previous_block_header = header.clone();
        let block = SerialBlock { header, transactions };

        assert!(consensus.verify_transactions(block.transactions.clone()).await);

        let block_reward = get_block_reward(consensus.storage.canon().await.unwrap().block_height as u32);

        // dummy outputs have 0 balance, coinbase only pays the miner
        assert_eq!(coinbase_records.len(), 2);

        let coinbase_record_0 = <DPCRecord<Components> as VMRecord>::deserialize(&coinbase_records[0]).unwrap();
        let coinbase_record_1 = <DPCRecord<Components> as VMRecord>::deserialize(&coinbase_records[1]).unwrap();

        assert!(!coinbase_record_0.is_dummy());
        assert!(coinbase_record_1.is_dummy());
        assert_eq!(coinbase_record_0.value(), block_reward.0 as u64);
        assert_eq!(coinbase_record_1.value(), 0);

        let mut joint_serial_numbers = vec![];
        for record in &[coinbase_record_0, coinbase_record_1] {
            let (sn, _) = record
                .to_serial_number(
                    &consensus.dpc.system_parameters.account_signature,
                    &miner_acc.private_key,
                )
                .unwrap();
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn].unwrap());
        }

        println!("Verifying and receiving the block");
        assert!(consensus.receive_block(block.clone()).await);
        assert_eq!(consensus.storage.canon().await.unwrap().block_height, 1);

        // Add new block spending records from the previous block

        // INPUTS

        let old_account_private_keys = vec![miner_acc.private_key; Components::NUM_INPUT_RECORDS]
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<_>>();

        let old_records = coinbase_records;

        let new_birth_program_ids = vec![program.id(); Components::NUM_INPUT_RECORDS];

        // OUTPUTS

        let new_record_owners = vec![recipient.address; Components::NUM_OUTPUT_RECORDS];

        let new_death_program_ids = vec![program.id(); Components::NUM_OUTPUT_RECORDS];
        let new_is_dummy_flags = vec![false; Components::NUM_OUTPUT_RECORDS];
        let new_values = vec![10; Components::NUM_OUTPUT_RECORDS];
        let new_payloads = vec![RecordPayload::default(); Components::NUM_OUTPUT_RECORDS];

        let mut new_records = vec![];
        for j in 0..Components::NUM_OUTPUT_RECORDS {
            new_records.push(
                DPCRecord::new_full(
                    &consensus.dpc.system_parameters.serial_number_nonce,
                    &consensus.dpc.system_parameters.record_commitment,
                    new_record_owners[j].clone(),
                    new_is_dummy_flags[j],
                    new_values[j],
                    new_payloads[j].clone(),
                    new_birth_program_ids[j].clone(),
                    new_death_program_ids[j].clone(),
                    j as u8,
                    joint_serial_numbers.clone(),
                    &mut thread_rng(),
                )
                .unwrap()
                .serialize()
                .unwrap(),
            );
        }

        // Memo is a dummy for now

        let memo = [6u8; 32];

        println!("Create a payment transaction");
        // Create the transaction
        let response = consensus
            .create_transaction(CreateTransactionRequest {
                old_records,
                old_account_private_keys,
                new_records,
                memo,
            })
            .await
            .unwrap();

        assert_eq!(response.records.len(), 2);

        let spend_record_0 = <DPCRecord<Components> as VMRecord>::deserialize(&response.records[0]).unwrap();
        let spend_record_1 = <DPCRecord<Components> as VMRecord>::deserialize(&response.records[1]).unwrap();
        let transaction = response.transaction;

        assert!(!spend_record_0.is_dummy());
        assert!(!spend_record_1.is_dummy());
        assert_eq!(spend_record_0.value(), 10);
        assert_eq!(spend_record_1.value(), 10);
        assert_eq!(transaction.value_balance.0, (block_reward.0 - 20) as i64);

        assert!(consensus.verify_transactions(vec![transaction.clone()]).await);

        println!("Create a new block with the payment transaction");
        let (transactions, new_coinbase_records) = miner.establish_block(vec![transaction.clone()]).await.unwrap();

        assert!(consensus.verify_transactions(transactions.clone()).await);

        let header = miner.find_block(&transactions, &previous_block_header).unwrap();
        let new_block = SerialBlock { header, transactions };
        let new_block_reward = get_block_reward(consensus.storage.canon().await.unwrap().block_height as u32);

        assert_eq!(new_coinbase_records.len(), 2);
        let new_coinbase_record_0 = <DPCRecord<Components> as VMRecord>::deserialize(&new_coinbase_records[0]).unwrap();
        let new_coinbase_record_1 = <DPCRecord<Components> as VMRecord>::deserialize(&new_coinbase_records[1]).unwrap();

        assert!(!new_coinbase_record_0.is_dummy());
        assert!(new_coinbase_record_1.is_dummy());
        assert_eq!(
            new_coinbase_record_0.value(),
            (new_block_reward.0 + block_reward.0 - 20) as u64
        );
        assert_eq!(new_coinbase_record_1.value(), 0);

        println!("Verify and receive the block with the new payment transaction");

        assert!(consensus.receive_block(new_block).await);

        assert_eq!(consensus.storage.canon().await.unwrap().block_height, 2);

        for record in &new_coinbase_records {
            consensus.storage.store_records(&[record.clone()]).await.unwrap();

            let reconstruct_record = consensus
                .storage
                .get_record(record.commitment.clone())
                .await
                .unwrap()
                .unwrap();

            assert_eq!(&reconstruct_record, record);
        }
    }
}
