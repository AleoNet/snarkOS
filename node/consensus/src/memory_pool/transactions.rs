// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

use indexmap::map;

impl<N: Network> MemoryPool<N> {
    /// Returns `true` if the given unconfirmed transaction exists in the memory pool.
    pub fn contains_unconfirmed_transaction(&self, transaction_id: N::TransactionID) -> bool {
        self.unconfirmed_transactions.read().contains_key(&transaction_id)
    }

    /// Returns the number of unconfirmed transactions in the memory pool.
    pub fn num_unconfirmed_transactions(&self) -> usize {
        self.unconfirmed_transactions.read().len()
    }

    /// Returns the unconfirmed transactions in the memory pool.
    pub fn unconfirmed_transactions(&self) -> Vec<Transaction<N>> {
        self.unconfirmed_transactions.read().values().cloned().collect::<Vec<_>>()
    }

    /// Returns a candidate set of unconfirmed transactions for inclusion in a block.
    pub fn candidate_transactions<C: ConsensusStorage<N>>(&self, consensus: &Consensus<N, C>) -> Vec<Transaction<N>> {
        // TODO (raychu86): Add more sophisticated logic for transaction selection.

        // Add the transactions from the memory pool that do not have input collisions.
        let mut transactions = Vec::new();
        let mut input_ids = Vec::new();
        let mut output_ids = Vec::new();

        'outer: for transaction in self.unconfirmed_transactions.read().values() {
            // Ensure the transaction is not a fee transaction.
            if matches!(transaction, Transaction::Fee(..)) {
                continue;
            }

            // Ensure the transaction is well-formed.
            if consensus.check_transaction_basic(transaction, None).is_err() {
                continue;
            }

            // Ensure the input IDs are unique.
            for input_id in transaction.input_ids() {
                if input_ids.contains(&input_id) {
                    continue 'outer;
                }
            }
            // Ensure the output IDs are unique.
            for output_id in transaction.output_ids() {
                if output_ids.contains(&output_id) {
                    continue 'outer;
                }
            }

            transactions.push(transaction.clone());
            input_ids.extend(transaction.input_ids());
            output_ids.extend(transaction.output_ids());
        }

        transactions
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub fn add_unconfirmed_transaction(&self, transaction: &Transaction<N>) -> bool {
        // Acquire the write lock on the unconfirmed transactions.
        let mut unconfirmed_transactions = self.unconfirmed_transactions.write();

        // Ensure the transaction does not already exist in the memory pool.
        match unconfirmed_transactions.entry(transaction.id()) {
            map::Entry::Vacant(entry) => {
                // Add the transaction to the memory pool.
                entry.insert(transaction.clone());
                debug!("✉️  Added transaction '{}' to the memory pool", transaction.id());
                true
            }
            map::Entry::Occupied(_) => {
                trace!("Transaction '{}' already exists in memory pool", transaction.id());
                false
            }
        }
    }

    /// Clears the memory pool of unconfirmed transactions that are now invalid.
    pub fn clear_invalid_transactions<C: ConsensusStorage<N>>(&self, consensus: &Consensus<N, C>) {
        self.unconfirmed_transactions.write().retain(|transaction_id, transaction| {
            // Ensure the transaction is not a fee transaction.
            if matches!(transaction, Transaction::Fee(..)) {
                trace!("Removed transaction '{transaction_id}' from the memory pool");
                return false;
            }
            // Ensure the transaction is valid.
            match consensus.check_transaction_basic(transaction, None) {
                Ok(_) => true,
                Err(_) => {
                    trace!("Removed transaction '{transaction_id}' from the memory pool");
                    false
                }
            }
        });
    }

    /// Clears the memory pool of all unconfirmed transactions.
    pub fn clear_unconfirmed_transactions(&self) {
        self.unconfirmed_transactions.write().clear();
    }
}
