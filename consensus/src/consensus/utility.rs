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

use snarkos_storage::PrivateKey;

use super::*;

impl Consensus {
    /// Generate a coinbase transaction given candidate block transactions
    #[allow(clippy::too_many_arguments)]
    pub async fn create_coinbase_transaction(
        &self,
        block_num: u32,
        transactions: &[SerialTransaction],
        program_vk_hash: Vec<u8>,
        new_birth_program_ids: Vec<Vec<u8>>,
        new_death_program_ids: Vec<Vec<u8>>,
        recipients: Vec<Address>,
    ) -> Result<TransactionResponse, ConsensusError> {
        let mut rng = thread_rng();
        let mut total_value_balance = crate::get_block_reward(block_num);

        for transaction in transactions.iter() {
            let tx_value_balance = transaction.value_balance;

            if tx_value_balance.is_negative() {
                return Err(ConsensusError::CoinbaseTransactionAlreadyExists());
            }

            total_value_balance = total_value_balance.add(transaction.value_balance);
        }

        // Generate a new account that owns the dummy input records
        let new_account = Account::<Components>::new(
            &self.dpc.system_parameters.account_signature,
            &self.dpc.system_parameters.account_commitment,
            &self.dpc.system_parameters.account_encryption,
            &mut rng,
        )
        .unwrap();

        // Generate dummy input records having as address the genesis address.
        let old_account_private_keys = vec![new_account.private_key.clone(); Components::NUM_INPUT_RECORDS]
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<_>>();
        let mut old_records = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut joint_serial_numbers = vec![];

        for i in 0..Components::NUM_INPUT_RECORDS {
            let sn_nonce_input: [u8; 4] = rng.gen();

            let old_sn_nonce = <Components as DPCComponents>::SerialNumberNonceCRH::hash(
                &self.dpc.system_parameters.serial_number_nonce,
                &sn_nonce_input,
            )?;

            let old_record = DPCRecord::new(
                &self.dpc.system_parameters.record_commitment,
                new_account.address.clone(),
                true, // The input record is dummy
                0,
                Payload::default(),
                // Filler program input
                program_vk_hash.clone(),
                program_vk_hash.clone(),
                old_sn_nonce,
                &mut rng,
            )?;

            let (sn, _) = old_record.to_serial_number(
                &self.dpc.system_parameters.account_signature,
                &old_account_private_keys[i],
            )?;
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn]?);

            old_records.push(old_record.serialize()?);
        }

        let new_is_dummy_flags = [vec![false], vec![true; Components::NUM_OUTPUT_RECORDS - 1]].concat();
        let new_values = [vec![total_value_balance.0 as u64], vec![
            0;
            Components::NUM_OUTPUT_RECORDS
                - 1
        ]]
        .concat();
        let new_payloads = vec![Payload::default(); Components::NUM_OUTPUT_RECORDS];

        let mut new_records = vec![];
        for j in 0..Components::NUM_OUTPUT_RECORDS {
            new_records.push(
                DPCRecord::new_full(
                    &self.dpc.system_parameters.serial_number_nonce,
                    &self.dpc.system_parameters.record_commitment,
                    recipients[j].clone().into(),
                    new_is_dummy_flags[j],
                    new_values[j],
                    new_payloads[j].clone(),
                    new_birth_program_ids[j].clone(),
                    new_death_program_ids[j].clone(),
                    j as u8,
                    joint_serial_numbers.clone(),
                    &mut rng,
                )?
                .serialize()?,
            );
        }

        let memo: [u8; 32] = rng.gen();

        self.create_transaction(CreateTransactionRequest {
            old_records,
            old_account_private_keys: old_account_private_keys.into_iter().map(|x| x.into()).collect(),
            new_records,
            memo,
        })
        .await
    }

    pub fn calculate_joint_serial_numbers(
        &self,
        records: &[SerialRecord],
        private_keys: &[PrivateKey],
    ) -> Result<Vec<u8>> {
        assert!(records.len() == private_keys.len());
        let mut joint_serial_numbers = vec![];

        for (record, key) in records.iter().zip(private_keys.iter()) {
            let (sn, _) = <DPCRecord<Components> as VMRecord>::deserialize(record)?
                .to_serial_number(&self.dpc.system_parameters.account_signature, key.into_ref())?;
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn]?);
        }
        Ok(joint_serial_numbers)
    }

    pub fn make_dummy_record(
        &self,
        joint_serial_numbers: &[u8],
        position: u8,
        new_record_owner: Address,
        value: AleoAmount,
        payload: Payload,
    ) -> Result<SerialRecord> {
        Ok(DPCRecord::new_full(
            &self.dpc.system_parameters.serial_number_nonce,
            &self.dpc.system_parameters.record_commitment,
            new_record_owner.into(),
            true,
            value.0.try_into()?,
            payload,
            self.dpc.noop_program.id(),
            self.dpc.noop_program.id(),
            position,
            joint_serial_numbers.to_vec(),
            &mut thread_rng(),
        )?
        .serialize()?)
    }
}
