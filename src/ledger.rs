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

use crate::{state::LedgerState, storage::Storage};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
use std::path::Path;

pub struct Ledger<N: Network> {
    /// The canonical chain of block hashes.
    canon: LedgerState<N>,
    /// The pool of unconfirmed transactions.
    memory_pool: MemoryPool<N>,
}

impl<N: Network> Ledger<N> {
    /// Initializes a new instance of the ledger.
    pub fn open<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        // Open storage.
        let context = N::NETWORK_ID;
        let storage = S::open(path, context)?;

        // let value = storage.export()?;
        // println!("{}", value);
        // let storage_2 = S::open(".ledger_2", context)?;
        // storage_2.import(value)?;

        Ok(Self {
            canon: LedgerState::open::<S>(storage)?,
            memory_pool: MemoryPool::new(),
        })
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.canon.latest_block_height()
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.canon.latest_block_hash()
    }

    /// Returns the latest ledger root.
    pub fn latest_ledger_root(&self) -> N::LedgerRoot {
        self.canon.latest_ledger_root()
    }

    /// Returns the latest block timestamp.
    pub fn latest_block_timestamp(&self) -> Result<i64> {
        self.canon.latest_block_timestamp()
    }

    /// Returns the latest block difficulty target.
    pub fn latest_block_difficulty_target(&self) -> Result<u64> {
        self.canon.latest_block_difficulty_target()
    }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> Result<BlockHeader<N>> {
        self.canon.latest_block_header()
    }

    /// Returns the transactions from the latest block.
    pub fn latest_block_transactions(&self) -> Result<Transactions<N>> {
        self.canon.latest_block_transactions()
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Result<Block<N>> {
        self.canon.latest_block()
    }

    /// Returns `true` if the given ledger root exists in storage.
    pub fn contains_ledger_root(&self, ledger_root: &N::LedgerRoot) -> Result<bool> {
        self.canon.contains_ledger_root(ledger_root)
    }

    /// Returns `true` if the given block height exists in storage.
    pub fn contains_block_height(&self, block_height: u32) -> Result<bool> {
        self.canon.contains_block_height(block_height)
    }

    /// Returns `true` if the given block hash exists in storage.
    pub fn contains_block_hash(&self, block_hash: &N::BlockHash) -> Result<bool> {
        self.canon.contains_block_hash(block_hash)
    }

    /// Returns `true` if the given transaction ID exists in storage.
    pub fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        self.canon.contains_transaction(transaction_id)
    }

    /// Returns `true` if the given serial number exists in storage.
    pub fn contains_serial_number(&self, serial_number: &N::SerialNumber) -> Result<bool> {
        self.canon.contains_serial_number(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub fn contains_commitment(&self, commitment: &N::Commitment) -> Result<bool> {
        self.canon.contains_commitment(commitment)
    }

    /// Returns `true` if the given ciphertext ID exists in storage.
    pub fn contains_ciphertext_id(&self, ciphertext_id: &N::CiphertextID) -> Result<bool> {
        self.canon.contains_ciphertext_id(ciphertext_id)
    }

    /// Returns the record ciphertext for a given ciphertext ID.
    pub fn get_ciphertext(&self, ciphertext_id: &N::CiphertextID) -> Result<RecordCiphertext<N>> {
        self.canon.get_ciphertext(ciphertext_id)
    }

    /// Returns the transition for a given transition ID.
    pub fn get_transition(&self, transition_id: &N::TransitionID) -> Result<Transition<N>> {
        self.canon.get_transition(transition_id)
    }

    /// Returns the transaction for a given transaction ID.
    pub fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        self.canon.get_transaction(transaction_id)
    }

    /// Returns the block hash for the given block height.
    pub fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.canon.get_block_hash(block_height)
    }

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.canon.get_previous_block_hash(block_height)
    }

    /// Returns the block header for the given block height.
    pub fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        self.canon.get_block_header(block_height)
    }

    /// Returns the transactions from the block of the given block height.
    pub fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        self.canon.get_block_transactions(block_height)
    }

    /// Returns the block for a given block height.
    pub fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        self.canon.get_block(block_height)
    }

    /// Adds the given block as the next block in the ledger to storage.
    pub fn add_next_block(&mut self, block: &Block<N>) -> Result<()> {
        self.canon.add_next_block(block)
    }
}
