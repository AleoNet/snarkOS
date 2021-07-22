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

use snarkvm_dpc::testnet1::instantiated::{Testnet1DPC, Testnet1Transaction};
use snarkos_storage::VMRecord;
use std::io::Result as IoResult;

use crate::DeserializedLedger;

use super::*;

impl ConsensusInner {
    pub(super) fn receive_transaction(&mut self, transaction: Box<SerialTransaction>) -> bool {
        if transaction.value_balance.is_negative() {
            error!("Received a transaction that was a coinbase transaction");
            return false;
        }

        match self.verify_transactions(std::iter::once(&*transaction)) {
            Ok(true) => (),
            Ok(false) => {
                error!("Received a transaction that was invalid");
                return false;
            }
            Err(e) => {
                error!(
                    "failed to verify transaction -- note this does not mean the transaction was valid or invalid: {:?}",
                    e
                );
                return false;
            }
        }
        match self.memory_pool.insert(&self.ledger, *transaction) {
            Ok(Some(digest)) => {
                debug!("pushed transaction into memory pool: {:02X?}", &digest[..]);
                true
            }
            Ok(None) => false,
            Err(e) => {
                error!("failed pushing transaction into memory pool: {:?}", e);
                false
            }
        }
    }

    /// Check if the transactions are valid.
    pub(super) fn verify_transactions<'a>(
        &self,
        transactions: impl Iterator<Item = &'a SerialTransaction>,
    ) -> Result<bool, ConsensusError> {
        let mut deserialized = vec![];
        for transaction in transactions {
            if !self
                .public
                .parameters
                .authorized_inner_snark_ids
                .iter()
                .any(|x| x[..] == transaction.inner_circuit_id[..])
            {
                return Ok(false);
            }
            deserialized.push(Testnet1Transaction::deserialize(transaction)?);
        }

        Ok(self.public.dpc.verify_transactions(
            &deserialized[..],
            &self.ledger.deserialize::<Components>(),
        ))
    }

    /// Generate a transaction by spending old records and specifying new record attributes
    pub(super) fn create_transaction(
        &self,
        request: CreateTransactionRequest,
    ) -> Result<TransactionResponse, ConsensusError> {
        let mut rng = thread_rng();
        // Offline execution to generate a DPC transaction
        let old_private_keys: Vec<_> = request.old_account_private_keys.into_iter().map(|x| x.into()).collect();
        let transaction_kernel = <Testnet1DPC as DPCScheme<DeserializedLedger<'_, Components>>>::execute_offline_phase(
            &self.public.dpc,
            &old_private_keys,
            request
                .old_records
                .iter()
                .map(|x| <DPCRecord<Components> as VMRecord>::deserialize(x))
                .collect::<Result<Vec<_>, _>>()?,
            request.new_records.into_iter().map(|x| DPCRecord::<Components>::deserialize(&x)).collect::<IoResult<Vec<_>>>()?,
            request.memo,
            &mut rng,
        )?;

        // Construct the program proofs
        let program_proofs = ConsensusParameters::generate_program_proofs(
            &self.public.dpc,
            &transaction_kernel,
            &mut rng,
        )?;

        // Online execution to generate a DPC transaction
        let (new_records, transaction) = self.public.dpc.execute_online_phase(
            &old_private_keys,
            transaction_kernel,
            program_proofs,
            &self.ledger.deserialize::<Components>(),
            &mut rng,
        )?;

        let serialized_records = new_records.iter().map(|x| x.serialize()).collect::<Result<Vec<_>>>()?;

        Ok(TransactionResponse {
            records: serialized_records,
            transaction: transaction.serialize()?,
        })
    }

    /// Generate a transaction by spending old records and specifying new record attributes
    pub(super) fn create_partial_transaction(
        &self,
        request: CreatePartialTransactionRequest,
    ) -> Result<TransactionResponse, ConsensusError> {
        let mut rng = thread_rng();
        // Offline execution to generate a DPC transaction
        let transaction_kernel: Box<TransactionKernel<Components>> = request
            .kernel
            .downcast()
            .expect("illegal kernel passed to create partial transaction");
        let old_private_keys: Vec<_> = request.old_account_private_keys.into_iter().map(|x| x.into()).collect();

        // Construct the program proofs
        let program_proofs = ConsensusParameters::generate_program_proofs(
            &self.public.dpc,
            &*transaction_kernel,
            &mut rng,
        )?;

        // Online execution to generate a DPC transaction
        let (new_records, transaction) = self.public.dpc.execute_online_phase(
            &old_private_keys,
            *transaction_kernel,
            program_proofs,
            &self.ledger.deserialize::<Components>(),
            &mut rng,
        )?;

        let serialized_records = new_records.iter().map(|x| x.serialize()).collect::<Result<Vec<_>>>()?;

        Ok(TransactionResponse {
            records: serialized_records,
            transaction: transaction.serialize()?,
        })
    }
}
