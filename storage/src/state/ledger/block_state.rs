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

use crate::{
    state::ledger::{transaction_state::TransactionState, Metadata},
    storage::{DataID, DataMap, MapRead, MapReadWrite, Storage, StorageAccess, StorageReadWrite},
};
use snarkvm::{
    circuit::Aleo,
    compiler::{Block, Header, Transaction, Transactions, Transition},
    console::types::field::Field,
    prelude::*,
};

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::{collections::HashSet, marker::PhantomData};

#[derive(Clone, Debug)]
pub(crate) struct BlockState<N: Network, SA: StorageAccess> {
    pub(crate) block_heights: DataMap<u32, N::BlockHash, SA>,
    pub(crate) block_headers: DataMap<N::BlockHash, Header<N>, SA>,
    pub(crate) block_transactions: DataMap<N::BlockHash, Vec<N::TransactionID>, SA>,
    pub(crate) transactions: TransactionState<N, SA>,
}

impl<N: Network, SA: StorageAccess> BlockState<N, SA> {
    /// Initializes a new instance of `BlockState`.
    pub(crate) fn open<S: Storage<Access = SA>>(storage: S) -> Result<Self> {
        Ok(Self {
            block_heights: storage.open_map(DataID::BlockHeights)?,
            block_headers: storage.open_map(DataID::BlockHeaders)?,
            block_transactions: storage.open_map(DataID::BlockTransactions)?,
            transactions: TransactionState::open(storage)?,
        })
    }

    /// Returns `true` if the given block height exists in storage.
    pub(crate) fn contains_block_height(&self, block_height: u32) -> Result<bool> {
        self.block_heights.contains_key(&block_height)
    }

    /// Returns `true` if the given block hash exists in storage.
    pub(crate) fn contains_block_hash(&self, block_hash: &N::BlockHash) -> Result<bool> {
        self.block_headers.contains_key(block_hash)
    }

    /// Returns `true` if the given transaction ID exists in storage.
    pub(crate) fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        self.transactions.contains_transaction(transaction_id)
    }

    /// Returns `true` if the given serial number exists in storage.
    pub(crate) fn contains_serial_number(&self, serial_number: &Field<N>) -> Result<bool> {
        self.transactions.contains_serial_number(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub(crate) fn contains_commitment(&self, commitment: &Field<N>) -> Result<bool> {
        self.transactions.contains_commitment(commitment)
    }

    // /// Returns the record ciphertext for a given commitment.
    // fn get_ciphertext(&self, commitment: &Field<N>) -> Result<N::RecordCiphertext> {
    //     self.transactions.get_ciphertext(commitment)
    // }

    /// Returns the transition for a given transition ID.
    pub(crate) fn get_transition(&self, transition_id: &Field<N>) -> Result<Transition<N>> {
        self.transactions.get_transition(transition_id)
    }

    /// Returns the transaction for a given transaction ID.
    pub(crate) fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        self.transactions.get_transaction(transaction_id)
    }

    /// Returns the transaction metadata for a given transaction ID.
    pub(crate) fn get_transaction_metadata(&self, transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        self.transactions.get_transaction_metadata(transaction_id)
    }

    // /// Returns the cumulative weight up to a given block height (inclusive) for the canonical chain.
    // fn get_cumulative_weight(&self, block_height: u32) -> Result<u128> {
    //     Ok(self.get_block_header(block_height)?.cumulative_weight())
    // }

    /// Returns the block height for the given block hash.
    pub(crate) fn get_block_height(&self, block_hash: &N::BlockHash) -> Result<u32> {
        match self.block_headers.get(block_hash)? {
            Some(block_header) => Ok(block_header.height()),
            None => Err(anyhow!("Block {} missing from block headers map", block_hash)),
        }
    }

    /// Returns the block hash for the given block height.
    pub(crate) fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.get_previous_block_hash(block_height + 1)
    }

    /// Returns the block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    pub(crate) fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        // Ensure the starting block height is less than the ending block height.
        if start_block_height > end_block_height {
            return Err(anyhow!("Invalid starting and ending block heights"));
        }

        (start_block_height..=end_block_height)
            .into_par_iter()
            .map(|height| self.get_block_hash(height))
            .collect()
    }

    /// Returns the previous block hash for the given block height.
    pub(crate) fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        match block_height == 0 {
            true => Ok(N::BlockHash::default()), // Previous block hash of the genesis block.
            false => match self.block_heights.get(&(block_height - 1))? {
                Some(block_hash) => Ok(block_hash),
                None => Err(anyhow!("Block {} missing in block heights map", block_height - 1)),
            },
        }
    }

    /// Returns the block header for the given block height.
    pub(crate) fn get_block_header(&self, block_height: u32) -> Result<Header<N>> {
        // Retrieve the block hash.
        let block_hash = self.get_block_hash(block_height)?;

        match self.block_headers.get(&block_hash)? {
            Some(block_header) => Ok(block_header),
            None => Err(anyhow!("Block {} missing from block headers map", block_hash)),
        }
    }

    /// Returns the block headers from the given `start_block_height` to `end_block_height` (inclusive).
    pub(crate) fn get_block_headers(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Header<N>>> {
        // Ensure the starting block height is less than the ending block height.
        if start_block_height > end_block_height {
            return Err(anyhow!("Invalid starting and ending block heights"));
        }

        (start_block_height..=end_block_height)
            .into_par_iter()
            .map(|height| self.get_block_header(height))
            .collect()
    }

    /// Returns the number of all block headers belonging to canonical blocks.
    pub(crate) fn get_block_header_count(&self) -> Result<u32> {
        let block_hashes = self.block_heights.values().collect::<HashSet<_>>();
        let count = self.block_headers.keys().filter(|hash| block_hashes.contains(hash)).count();
        Ok(count as u32)
    }

    /// Returns the transactions from the block of the given block height.
    pub(crate) fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        // Retrieve the block hash.
        let block_hash = self.get_block_hash(block_height)?;

        // Retrieve the block transaction IDs.
        let transaction_ids = match self.block_transactions.get(&block_hash)? {
            Some(transaction_ids) => transaction_ids,
            None => return Err(anyhow!("Block {} missing from block transactions map", block_hash)),
        };

        // Retrieve the block transactions.
        let transactions = {
            let mut transactions = Vec::with_capacity(transaction_ids.len());
            for transaction_id in transaction_ids.iter() {
                transactions.push(self.transactions.get_transaction(transaction_id)?)
            }
            Transactions::from(&transactions)
        };

        Ok(transactions)
    }

    /// Returns the block for a given block height.
    pub(crate) fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        // Retrieve the previous block hash.
        let previous_block_hash = self.get_previous_block_hash(block_height)?;
        // Retrieve the block header.
        let block_header = self.get_block_header(block_height)?;
        // Retrieve the block transactions.
        let transactions = self.get_block_transactions(block_height)?;

        // TODO (raychu86): Add support for signatures.
        // Ok(Block::from(previous_block_hash, block_header, transactions)?)

        Err(anyhow!("Can't get block {block_height}, signatures are not yet supported"))
    }

    /// Returns the blocks from the given `start_block_height` to `end_block_height` (inclusive).
    pub(crate) fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        // Ensure the starting block height is less than the ending block height.
        if start_block_height > end_block_height {
            return Err(anyhow!("Invalid starting and ending block heights"));
        }

        (start_block_height..=end_block_height)
            .into_par_iter()
            .map(|height| self.get_block(height))
            .collect()
    }

    /// Returns the state root in the block header of the given block height.
    pub(crate) fn get_previous_state_root(&self, block_height: u32) -> Result<Field<N>> {
        // Retrieve the block header.
        let block_header = self.get_block_header(block_height)?;
        // Return the state root in the block header.
        Ok(*block_header.previous_state_root())
    }
}

impl<N: Network, SA: StorageReadWrite> BlockState<N, SA> {
    /// Adds the given block to storage.
    pub(crate) fn add_block(&self, block: &Block<N>, batch: Option<usize>) -> Result<()> {
        // Ensure the block does not exist.
        let block_height = block.header().height();
        if self.block_heights.contains_key(&block_height)? {
            Err(anyhow!("Block {} already exists in storage", block_height))
        } else {
            let block_hash = block.hash();
            let block_header = block.header();
            let transactions = block.transactions();
            let transaction_ids = transactions.transaction_ids().cloned().collect::<Vec<_>>();

            // Insert the block height.
            self.block_heights.insert(&block_height, &block_hash, batch)?;
            // Insert the block header.
            self.block_headers.insert(&block_hash, block_header, batch)?;
            // Insert the block transactions.
            self.block_transactions.insert(&block_hash, &transaction_ids, batch)?;
            // Insert the transactions.
            for (index, (_transaction_id, transaction)) in (*transactions).iter().enumerate() {
                let metadata = Metadata::<N>::new(block_height, block_hash, block.header().timestamp(), index as u16);
                self.transactions.add_transaction(transaction, metadata, batch)?;
            }

            Ok(())
        }
    }

    /// Removes the given block height from storage.
    pub(crate) fn remove_block(&self, block_height: u32, batch: Option<usize>) -> Result<()> {
        // Ensure the block height is not the genesis block.
        if block_height == 0 {
            Err(anyhow!("Block {} cannot be removed from storage", block_height))
        }
        // Remove the block at the given block height.
        else {
            // Retrieve the block hash.
            let block_hash = match self.block_heights.get(&block_height)? {
                Some(block_hash) => block_hash,
                None => return Err(anyhow!("Block {} missing from block heights map", block_height)),
            };

            // Retrieve the block header.
            let block_header = match self.block_headers.get(&block_hash)? {
                Some(block_header) => block_header,
                None => return Err(anyhow!("Block {} missing from block headers map", block_hash)),
            };
            // Retrieve the block transaction IDs.
            let transaction_ids = match self.block_transactions.get(&block_hash)? {
                Some(transaction_ids) => transaction_ids,
                None => return Err(anyhow!("Block {} missing from block transactions map", block_hash)),
            };

            // Retrieve the block height.
            let block_height = block_header.height();

            // Remove the block height.
            self.block_heights.remove(&block_height, batch)?;
            // Remove the block header.
            self.block_headers.remove(&block_hash, batch)?;
            // Remove the block transactions.
            self.block_transactions.remove(&block_hash, batch)?;
            // Remove the transactions.
            for transaction_ids in transaction_ids.iter() {
                self.transactions.remove_transaction(transaction_ids, batch)?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state::ledger::test_helpers::{sample_genesis_block, CurrentNetwork},
        storage::{
            rocksdb::{tests::temp_dir, RocksDB},
            ReadWrite,
            Storage,
        },
    };

    #[test]
    fn test_open_block_state() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let _block_state = BlockState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open block state");
    }

    #[test]
    fn test_insert_and_contains_block() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let block_state = BlockState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open block state");

        let block = sample_genesis_block();

        // Insert the block.
        block_state.add_block(&block, None).expect("Failed to add block");

        // Check that the block is in storage.
        assert!(block_state.contains_block_hash(&block.hash()).unwrap());
        assert!(block_state.contains_block_height(block.header().height()).unwrap());

        // Check that each transaction is accounted for.
        for (transaction_id, _transaction) in (*block.transactions()).iter() {
            assert!(block_state.contains_transaction(&transaction_id).unwrap());
        }

        // Check that each commitment is accounted for.
        for commitment in block.transactions().commitments() {
            assert!(block_state.contains_commitment(commitment).unwrap());
        }

        // Check that each serial number is accounted for.
        for serial_number in block.transactions().serial_numbers() {
            assert!(block_state.contains_serial_number(serial_number).unwrap());
        }
    }

    #[test]
    fn test_insert_and_get_block() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let block_state = BlockState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open block state");

        let block = sample_genesis_block();

        // Insert the block.
        block_state.add_block(&block, None).expect("Failed to add block");

        // Assert that the block in storage is the same.
        let stored_block = block_state.get_block(block.header().height()).unwrap();
        assert_eq!(block, stored_block);

        let stored_block_hash = block_state.get_block_hash(block.header().height()).unwrap();
        assert_eq!(block.hash(), stored_block_hash);

        let stored_block_header = block_state.get_block_header(block.header().height()).unwrap();
        assert_eq!(block.header(), &stored_block_header);

        let stored_block_transactions = block_state.get_block_transactions(block.header().height()).unwrap();
        assert_eq!(block.transactions(), &stored_block_transactions);
    }

    #[test]
    fn test_insert_and_remove_block() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let block_state = BlockState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open block state");

        let block = sample_genesis_block();

        // Insert the block.
        block_state.add_block(&block, None).expect("Failed to add block");
        assert!(block_state.contains_block_hash(&block.hash()).unwrap());
        assert!(block_state.contains_block_height(block.header().height()).unwrap());

        // TODO (raychu86): Insert and remove a non-genesis block.
        // Remove the block.
        // block_state
        //     .remove_block(block.header().height(), None)
        //     .expect("Failed to remove block");
        // assert!(!block_state.contains_block_hash(&block.hash()).unwrap());
        // assert!(!block_state.contains_block_height(block.header().height()).unwrap());
    }
}
