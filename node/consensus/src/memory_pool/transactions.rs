// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::*;

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
            // Ensure the transaction is well-formed.
            if consensus.check_transaction_basic(transaction).is_err() {
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
        match !unconfirmed_transactions.contains_key(&transaction.id()) {
            true => {
                // Add the transaction to the memory pool.
                unconfirmed_transactions.insert(transaction.id(), transaction.clone());
                debug!("✉️  Added transaction '{}' to the memory pool", transaction.id());
                true
            }
            false => {
                trace!("Transaction '{}' already exists in memory pool", transaction.id());
                false
            }
        }
    }

    /// Clears the memory pool of unconfirmed transactions that are now invalid.
    pub fn clear_invalid_transactions<C: ConsensusStorage<N>>(&self, consensus: &Consensus<N, C>) {
        self.unconfirmed_transactions.write().retain(|transaction_id, transaction| {
            // Ensure the transaction is valid.
            match consensus.check_transaction_basic(transaction) {
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
