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

//! Transactions memory pool
//!
//! `MemoryPool` keeps a vector of transactions seen by the miner.

use crate::{error::ConsensusError, DynLedger};
use indexmap::{IndexMap, IndexSet};
use snarkos_storage::{Digest, SerialTransaction};
use snarkvm_dpc::BlockHeader;
use snarkvm_utilities::has_duplicates;

/// Stores a transaction and it's size in the memory pool.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MempoolEntry {
    size_in_bytes: usize,
    transaction: SerialTransaction,
}

/// Stores transactions received by the server.
/// Transaction entries will eventually be fetched by the miner and assembled into blocks.
#[derive(Debug, Default)]
pub struct MemoryPool {
    /// The mapping of all unconfirmed transaction IDs to their corresponding transaction data.
    transactions: IndexMap<Digest, MempoolEntry>,
    commitments: IndexSet<Digest>,
    serial_numbers: IndexSet<Digest>,
    memos: IndexSet<Digest>,
}

const BLOCK_HEADER_SIZE: usize = BlockHeader::size();
const COINBASE_TRANSACTION_SIZE: usize = 1490; // TODO Find the value for actual coinbase transaction size

impl MemoryPool {
    /// Initialize a new memory pool with no transactions
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds entry to memory pool if valid in the current ledger.
    pub fn insert(
        &mut self,
        ledger: &DynLedger,
        transaction: SerialTransaction,
    ) -> Result<Option<Digest>, ConsensusError> {
        let transaction_id: Digest = transaction.id.into();

        if has_duplicates(&transaction.old_serial_numbers)
            || has_duplicates(&transaction.new_commitments)
            || self.transactions.contains_key(&transaction_id)
        {
            return Ok(None);
        }

        for sn in &transaction.old_serial_numbers {
            if ledger.contains_serial(sn) || self.serial_numbers.contains(sn) {
                return Ok(None);
            }
        }

        for cm in &transaction.new_commitments {
            if ledger.contains_commitment(cm) || self.commitments.contains(cm) {
                return Ok(None);
            }
        }

        if ledger.contains_memo(&transaction.memorandum) || self.memos.contains(&transaction.memorandum) {
            return Ok(None);
        }

        for sn in &transaction.old_serial_numbers {
            self.serial_numbers.insert(sn.clone());
        }

        for cm in &transaction.new_commitments {
            self.commitments.insert(cm.clone());
        }

        self.memos.insert(transaction.memorandum.clone());

        self.transactions.insert(transaction_id.clone(), MempoolEntry {
            size_in_bytes: transaction.size(),
            transaction,
        });

        Ok(Some(transaction_id))
    }

    /// Cleanse the memory pool of outdated transactions.
    pub fn cleanse(&self, ledger: &DynLedger) -> Result<MemoryPool, ConsensusError> {
        let mut new_pool = Self::new();

        for (_, entry) in &self.transactions {
            new_pool.insert(ledger, entry.transaction.clone())?;
        }

        Ok(new_pool)
    }

    /// Removes transaction from memory pool based on the transaction id.
    pub fn remove(&mut self, transaction_id: &Digest) -> Result<Option<SerialTransaction>, ConsensusError> {
        match self.transactions.remove(transaction_id) {
            Some(entry) => {
                for commitment in &entry.transaction.new_commitments {
                    if !self.commitments.remove(commitment) {
                        panic!("missing commitment from memory pool during removal");
                    }
                }
                for serial in &entry.transaction.old_serial_numbers {
                    if !self.serial_numbers.remove(serial) {
                        panic!("missing serial from memory pool during removal");
                    }
                }
                if !self.memos.remove(&entry.transaction.memorandum) {
                    panic!("missing memo from memory pool during removal");
                }
                Ok(Some(entry.transaction))
            }
            None => Ok(None),
        }
    }

    /// Get candidate transactions for a new block.
    pub fn get_candidates(&self, max_size: usize) -> Vec<&SerialTransaction> {
        let max_size = max_size - (BLOCK_HEADER_SIZE + COINBASE_TRANSACTION_SIZE);

        let mut block_size = 0;
        let mut transactions = vec![];

        // TODO Change naive transaction selection
        for (_, entry) in self.transactions.iter() {
            if block_size + entry.size_in_bytes <= max_size {
                block_size += entry.size_in_bytes;
                transactions.push(&entry.transaction);
            }
        }

        transactions
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::MerkleLedger;

    use super::*;
    use snarkos_testing::sync::*;
    use snarkvm_algorithms::{MerkleParameters, CRH};
    use snarkvm_dpc::testnet1::{Testnet1Components, instantiated::Components};
    use snarkvm_parameters::{LedgerMerkleTreeParameters, Parameter};
    use snarkvm_utilities::FromBytes;

    // MemoryPool tests use TRANSACTION_2 because memory pools shouldn't store coinbase transactions

    fn mock_ledger() -> DynLedger {
        let ledger_parameters = {
            type Parameters = <Components as Testnet1Components>::MerkleParameters;
            let parameters: <<Parameters as MerkleParameters>::H as CRH>::Parameters =
                FromBytes::read_le(&LedgerMerkleTreeParameters::load_bytes().unwrap()[..]).unwrap();
            let crh = <Parameters as MerkleParameters>::H::from(parameters);
            Arc::new(Parameters::from(crh))
        };

        DynLedger(Box::new(
            MerkleLedger::new(ledger_parameters, &[], &[], &[], &[]).unwrap(),
        ))
    }

    #[tokio::test]
    async fn push() {
        let blockchain = mock_ledger();

        let mut mem_pool = MemoryPool::new();

        mem_pool.insert(&blockchain, TRANSACTION_2.clone()).unwrap();

        // assert_eq!(size, mem_pool.total_size_in_bytes.load(Ordering::SeqCst));
        assert_eq!(1, mem_pool.transactions.len());

        // Duplicate pushes don't do anything

        mem_pool.insert(&blockchain, TRANSACTION_2.clone()).unwrap();

        // assert_eq!(size, mem_pool.total_size_in_bytes.load(Ordering::SeqCst));
        assert_eq!(1, mem_pool.transactions.len());
    }

    #[tokio::test]
    async fn remove_entry() {
        let blockchain = mock_ledger();

        let mut mem_pool = MemoryPool::new();

        mem_pool.insert(&blockchain, TRANSACTION_2.clone()).unwrap();

        assert_eq!(1, mem_pool.transactions.len());
        // assert_eq!(size, mem_pool.total_size_in_bytes.load(Ordering::SeqCst));

        mem_pool.remove(&TRANSACTION_2.id.into()).unwrap();

        assert_eq!(0, mem_pool.transactions.len());
    }

    #[tokio::test]
    async fn remove_transaction_by_hash() {
        let blockchain = mock_ledger();

        let mut mem_pool = MemoryPool::new();

        mem_pool.insert(&blockchain, TRANSACTION_2.clone()).unwrap();

        assert_eq!(1, mem_pool.transactions.len());
        // assert_eq!(size, mem_pool.total_size_in_bytes.load(Ordering::SeqCst));

        mem_pool.remove(&TRANSACTION_2.id.into()).unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        // assert_eq!(0, mem_pool.total_size_in_bytes.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn get_candidates() {
        let blockchain = mock_ledger();

        let mut mem_pool = MemoryPool::new();

        mem_pool.insert(&blockchain, TRANSACTION_2.clone()).unwrap();

        let candidates = mem_pool.get_candidates(65536);

        assert!(candidates.iter().any(|x| *x == &*TRANSACTION_2));
    }

    #[tokio::test]
    async fn cleanse_memory_pool() {
        let mut blockchain = mock_ledger();

        let mut mem_pool = MemoryPool::new();
        mem_pool.insert(&blockchain, TRANSACTION_2.clone()).unwrap();

        assert_eq!(1, mem_pool.transactions.len());

        blockchain
            .extend(
                &TRANSACTION_2.new_commitments[..],
                &TRANSACTION_2.old_serial_numbers[..],
                &[TRANSACTION_2.memorandum.clone()],
            )
            .unwrap();

        let mem_pool = mem_pool.cleanse(&blockchain).unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        // assert_eq!(0, mem_pool.total_size_in_bytes.load(Ordering::SeqCst));
    }
}
