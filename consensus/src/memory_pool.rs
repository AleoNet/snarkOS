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

use crate::error::ConsensusError;
use indexmap::{IndexMap, IndexSet};
use snarkos_storage::{Digest, SerialTransaction};
use snarkvm_dpc::BlockHeader;

/// Stores a transaction and it's size in the memory pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MempoolEntry {
    pub(crate) size_in_bytes: usize,
    pub(crate) transaction: SerialTransaction,
}

/// Stores transactions received by the server.
/// Transaction entries will eventually be fetched by the miner and assembled into blocks.
#[derive(Debug, Default)]
pub struct MemoryPool {
    /// The mapping of all unconfirmed transaction IDs to their corresponding transaction data.
    pub(crate) transactions: IndexMap<Digest, MempoolEntry>,
    pub(crate) commitments: IndexSet<Digest>,
    pub(crate) serial_numbers: IndexSet<Digest>,
    pub(crate) memos: IndexSet<Digest>,
}

const BLOCK_HEADER_SIZE: usize = BlockHeader::size();
const COINBASE_TRANSACTION_SIZE: usize = 1490; // TODO Find the value for actual coinbase transaction size

impl MemoryPool {
    /// Initialize a new memory pool with no transactions
    #[inline]
    pub fn new() -> Self {
        Self::default()
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
