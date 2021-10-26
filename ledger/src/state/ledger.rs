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

use crate::storage::{DataMap, Map, Storage};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

const TWO_HOURS_UNIX: i64 = 7200;

#[derive(Clone, Debug)]
pub struct LedgerState<N: Network> {
    /// The current state of the ledger.
    latest_state: (u32, N::BlockHash, N::LedgerRoot),
    ledger_tree: Arc<Mutex<LedgerTree<N>>>,
    ledger_roots: DataMap<N::LedgerRoot, u32>,
    blocks: BlockState<N>,
}

impl<N: Network> LedgerState<N> {
    /// Initializes a new instance of `LedgerState`.
    pub fn open<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        // Open storage.
        let context = N::NETWORK_ID;
        let storage = S::open(path, context)?;

        // Retrieve the genesis block.
        let genesis = N::genesis_block();
        // Initialize the ledger.
        let mut ledger = Self {
            latest_state: (genesis.height(), genesis.block_hash(), genesis.ledger_root()),
            ledger_tree: Arc::new(Mutex::new(LedgerTree::<N>::new()?)),
            ledger_roots: storage.open_map("ledger_roots")?,
            blocks: BlockState::open(storage)?,
        };

        // Determine the latest block height.
        let latest_block_height = match ledger.ledger_roots.values().max() {
            Some(latest_block_height) => latest_block_height,
            None => 0u32,
        };

        // If this is new storage, initialize it with the genesis block.
        if latest_block_height == 0u32 && !ledger.blocks.contains_block_height(0u32)? {
            ledger.blocks.add_block(genesis)?;
        }

        // Retrieve each block from genesis to validate state.
        for block_height in 0..latest_block_height {
            // Ensure the ledger contains the block at given block height.
            let block = ledger.get_block(block_height)?;

            // Ensure the ledger roots match their expected block heights.
            match ledger.ledger_roots.get(&block.ledger_root())? {
                Some(height) => {
                    if height != block_height {
                        return Err(anyhow!("Ledger expected block {}, found block {}", block_height, height));
                    }
                }
                None => return Err(anyhow!("Ledger is missing ledger root for block {}", block_height)),
            }

            // Ensure the ledger tree matches the state of ledger roots.
            let candidate_ledger_root = ledger.ledger_tree.lock().unwrap().root();
            if block.ledger_root() != candidate_ledger_root {
                return Err(anyhow!("Ledger has incorrect ledger tree state at block {}", block_height));
            }
            ledger.ledger_tree.lock().unwrap().add(&block.block_hash())?;
        }

        // Update the latest state.
        let block = ledger.get_block(latest_block_height)?;
        ledger.latest_state = (block.height(), block.block_hash(), block.ledger_root());
        trace!("Loaded ledger from block {} ({})", block.height(), block.block_hash());

        // let value = storage.export()?;
        // println!("{}", value);
        // let storage_2 = S::open(".ledger_2", context)?;
        // storage_2.import(value)?;

        Ok(ledger)
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.latest_state.0
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.latest_state.1
    }

    /// Returns the latest ledger root.
    pub fn latest_ledger_root(&self) -> N::LedgerRoot {
        self.latest_state.2
    }

    /// Returns the latest block timestamp.
    pub fn latest_block_timestamp(&self) -> Result<i64> {
        Ok(self.latest_block_header()?.timestamp())
    }

    /// Returns the latest block difficulty target.
    pub fn latest_block_difficulty_target(&self) -> Result<u64> {
        Ok(self.latest_block_header()?.difficulty_target())
    }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> Result<BlockHeader<N>> {
        self.get_block_header(self.latest_block_height())
    }

    /// Returns the transactions from the latest block.
    pub fn latest_block_transactions(&self) -> Result<Transactions<N>> {
        self.get_block_transactions(self.latest_block_height())
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Result<Block<N>> {
        self.get_block(self.latest_block_height())
    }

    /// Returns `true` if the given ledger root exists in storage.
    pub fn contains_ledger_root(&self, ledger_root: &N::LedgerRoot) -> Result<bool> {
        Ok(*ledger_root == self.latest_ledger_root() || self.ledger_roots.contains_key(ledger_root)?)
    }

    /// Returns `true` if the given block height exists in storage.
    pub fn contains_block_height(&self, block_height: u32) -> Result<bool> {
        self.blocks.contains_block_height(block_height)
    }

    /// Returns `true` if the given block hash exists in storage.
    pub fn contains_block_hash(&self, block_hash: &N::BlockHash) -> Result<bool> {
        self.blocks.contains_block_hash(block_hash)
    }

    /// Returns `true` if the given transaction ID exists in storage.
    pub fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        self.blocks.contains_transaction(transaction_id)
    }

    /// Returns `true` if the given serial number exists in storage.
    pub fn contains_serial_number(&self, serial_number: &N::SerialNumber) -> Result<bool> {
        self.blocks.contains_serial_number(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub fn contains_commitment(&self, commitment: &N::Commitment) -> Result<bool> {
        self.blocks.contains_commitment(commitment)
    }

    /// Returns `true` if the given ciphertext ID exists in storage.
    pub fn contains_ciphertext_id(&self, ciphertext_id: &N::CiphertextID) -> Result<bool> {
        self.blocks.contains_ciphertext_id(ciphertext_id)
    }

    /// Returns the record ciphertext for a given ciphertext ID.
    pub fn get_ciphertext(&self, ciphertext_id: &N::CiphertextID) -> Result<RecordCiphertext<N>> {
        self.blocks.get_ciphertext(ciphertext_id)
    }

    /// Returns the transition for a given transition ID.
    pub fn get_transition(&self, transition_id: &N::TransitionID) -> Result<Transition<N>> {
        self.blocks.get_transition(transition_id)
    }

    /// Returns the transaction for a given transaction ID.
    pub fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        self.blocks.get_transaction(transaction_id)
    }

    /// Returns the block hash for the given block height.
    pub fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.blocks.get_block_hash(block_height)
    }

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.blocks.get_previous_block_hash(block_height)
    }

    /// Returns the block header for the given block height.
    pub fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        self.blocks.get_block_header(block_height)
    }

    /// Returns the transactions from the block of the given block height.
    pub fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        self.blocks.get_block_transactions(block_height)
    }

    /// Returns the block for a given block height.
    pub fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        self.blocks.get_block(block_height)
    }

    /// Adds the given block as the next block in the ledger to storage.
    pub fn add_next_block(&mut self, block: &Block<N>) -> Result<()> {
        // Ensure the block itself is valid.
        if !block.is_valid() {
            return Err(anyhow!("Block {} is invalid", block.height()));
        }

        // Retrieve the current block.
        let current_block = self.latest_block()?;

        // Ensure the block height increments by one.
        let block_height = block.height();
        if block_height != current_block.height() + 1 {
            return Err(anyhow!(
                "Block {} should have block height {}",
                block_height,
                current_block.height() + 1
            ));
        }

        // Ensure the previous block hash matches.
        if block.previous_block_hash() != current_block.block_hash() {
            return Err(anyhow!(
                "Block {} has an incorrect previous block hash in the canon chain",
                block_height
            ));
        }

        // Ensure the next block timestamp is within the declared time limit.
        let now = chrono::Utc::now().timestamp();
        if block.timestamp() > (now + TWO_HOURS_UNIX) {
            return Err(anyhow!("The given block timestamp exceeds the time limit"));
        }

        // Ensure the next block timestamp is after the current block timestamp.
        if block.timestamp() <= current_block.timestamp() {
            return Err(anyhow!("The given block timestamp is before the current timestamp"));
        }

        // Compute the expected difficulty target.
        let expected_difficulty_target =
            Blocks::<N>::compute_difficulty_target(current_block.timestamp(), current_block.difficulty_target(), block.timestamp());

        // Ensure the expected difficulty target is met.
        if block.difficulty_target() != expected_difficulty_target {
            return Err(anyhow!(
                "Block {} has an incorrect difficulty target. Found {}, but expected {}",
                block_height,
                block.difficulty_target(),
                expected_difficulty_target
            ));
        }

        // Ensure the block height does not already exist.
        if self.contains_block_height(block_height)? {
            return Err(anyhow!("Block {} already exists in the canon chain", block_height));
        }

        // Ensure the block hash does not already exist.
        if self.contains_block_hash(&block.block_hash())? {
            return Err(anyhow!("Block {} has a repeat block hash in the canon chain", block_height));
        }

        // Ensure the ledger root in the block matches the current ledger root.
        if block.ledger_root() != self.latest_ledger_root() {
            return Err(anyhow!("Block {} declares an incorrect ledger root", block_height));
        }

        // Ensure the canon chain does not already contain the given serial numbers.
        for serial_number in &block.serial_numbers() {
            if self.contains_serial_number(serial_number)? {
                return Err(anyhow!("Serial number {} already exists in the ledger", serial_number));
            }
        }

        // Ensure the canon chain does not already contain the given commitments.
        for commitment in &block.commitments() {
            if self.contains_commitment(commitment)? {
                return Err(anyhow!("Commitment {} already exists in the ledger", commitment));
            }
        }

        // Ensure each transaction in the given block is new to the canon chain.
        for transaction in block.transactions().iter() {
            // Ensure the transactions in the given block do not already exist.
            if self.contains_transaction(&transaction.transaction_id())? {
                return Err(anyhow!(
                    "Transaction {} in block {} has a duplicate transaction in the ledger",
                    transaction.transaction_id(),
                    block_height
                ));
            }

            // Ensure the transaction in the block references a valid past or current ledger root.
            if !self.contains_ledger_root(&transaction.ledger_root())? {
                return Err(anyhow!(
                    "Transaction {} in block {} references non-existent ledger root {}",
                    transaction.transaction_id(),
                    block_height,
                    &transaction.ledger_root()
                ));
            }
        }

        self.blocks.add_block(block)?;
        self.ledger_tree.lock().unwrap().add(&block.block_hash())?;
        self.ledger_roots.insert(&block.ledger_root(), &current_block.height())?;
        self.latest_state = (block_height, block.block_hash(), self.ledger_tree.lock().unwrap().root());
        Ok(())
    }

    /// Removes the latest block from storage, returning the removed block on success.
    pub fn remove_last_block(&mut self) -> Result<Block<N>> {
        let block = self.latest_block()?;
        let block_height = block.height();

        self.blocks.remove_block(block_height)?;
        self.ledger_roots.remove(&block.ledger_root())?;
        self.latest_state = match block_height == 0 {
            true => (0, block.previous_block_hash(), block.ledger_root()),
            false => (block_height - 1, block.previous_block_hash(), block.ledger_root()),
        };

        Ok(block)
    }
}

#[derive(Clone, Debug)]
struct BlockState<N: Network> {
    block_heights: DataMap<u32, N::BlockHash>,
    block_headers: DataMap<N::BlockHash, BlockHeader<N>>,
    block_transactions: DataMap<N::BlockHash, Vec<N::TransactionID>>,
    transactions: TransactionState<N>,
}

impl<N: Network> BlockState<N> {
    /// Initializes a new instance of `BlockState`.
    fn open<S: Storage>(storage: S) -> Result<Self> {
        Ok(Self {
            block_heights: storage.open_map("block_heights")?,
            block_headers: storage.open_map("block_headers")?,
            block_transactions: storage.open_map("block_transactions")?,
            transactions: TransactionState::open(storage)?,
        })
    }

    /// Returns `true` if the given block height exists in storage.
    fn contains_block_height(&self, block_height: u32) -> Result<bool> {
        self.block_heights.contains_key(&block_height)
    }

    /// Returns `true` if the given block hash exists in storage.
    fn contains_block_hash(&self, block_hash: &N::BlockHash) -> Result<bool> {
        self.block_headers.contains_key(block_hash)
    }

    /// Returns `true` if the given transaction ID exists in storage.
    fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        self.transactions.contains_transaction(transaction_id)
    }

    /// Returns `true` if the given serial number exists in storage.
    fn contains_serial_number(&self, serial_number: &N::SerialNumber) -> Result<bool> {
        self.transactions.contains_serial_number(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    fn contains_commitment(&self, commitment: &N::Commitment) -> Result<bool> {
        self.transactions.contains_commitment(commitment)
    }

    /// Returns `true` if the given ciphertext ID exists in storage.
    fn contains_ciphertext_id(&self, ciphertext_id: &N::CiphertextID) -> Result<bool> {
        self.transactions.contains_ciphertext_id(ciphertext_id)
    }

    /// Returns the record ciphertext for a given ciphertext ID.
    fn get_ciphertext(&self, ciphertext_id: &N::CiphertextID) -> Result<RecordCiphertext<N>> {
        self.transactions.get_ciphertext(ciphertext_id)
    }

    /// Returns the transition for a given transition ID.
    fn get_transition(&self, transition_id: &N::TransitionID) -> Result<Transition<N>> {
        self.transactions.get_transition(transition_id)
    }

    /// Returns the transaction for a given transaction ID.
    fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        self.transactions.get_transaction(transaction_id)
    }

    /// Returns the block hash for the given block height.
    fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.get_previous_block_hash(block_height + 1)
    }

    /// Returns the previous block hash for the given block height.
    fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        match block_height == 0 {
            true => Ok(N::genesis_block().previous_block_hash()),
            false => match self.block_heights.get(&(block_height - 1))? {
                Some(block_hash) => Ok(block_hash),
                None => return Err(anyhow!("Block {} missing from block heights map", block_height - 1)),
            },
        }
    }

    /// Returns the block header for the given block height.
    fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        // Retrieve the block hash.
        let block_hash = self.get_block_hash(block_height)?;

        match self.block_headers.get(&block_hash)? {
            Some(block_header) => Ok(block_header),
            None => return Err(anyhow!("Block {} missing from block headers map", block_hash)),
        }
    }

    /// Returns the transactions from the block of the given block height.
    fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
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
            Transactions::from(&transactions)?
        };

        Ok(transactions)
    }

    /// Returns the block for a given block height.
    fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        // Retrieve the previous block hash.
        let previous_block_hash = self.get_previous_block_hash(block_height)?;
        // Retrieve the block header.
        let block_header = self.get_block_header(block_height)?;
        // Retrieve the block transactions.
        let transactions = self.get_block_transactions(block_height)?;

        Block::from(previous_block_hash, block_header, transactions)
    }

    /// Adds the given block to storage.
    fn add_block(&self, block: &Block<N>) -> Result<()> {
        // Ensure the block does not exist.
        let block_height = block.height();
        if self.block_heights.contains_key(&block_height)? {
            Err(anyhow!("Block {} already exists in storage", block_height))
        } else {
            let block_hash = block.block_hash();
            let block_header = block.header();
            let transactions = block.transactions();
            let transaction_ids = transactions.transaction_ids().collect::<Vec<_>>();

            // Insert the block height.
            self.block_heights.insert(&block_height, &block_hash)?;
            // Insert the block header.
            self.block_headers.insert(&block_hash, &block_header)?;
            // Insert the block transactions.
            self.block_transactions.insert(&block_hash, &transaction_ids)?;
            // Insert the transactions.
            for (index, transaction) in transactions.iter().enumerate() {
                self.transactions.add_transaction(transaction, block_hash, index as u16)?;
            }

            Ok(())
        }
    }

    /// Removes the given block height from storage.
    fn remove_block(&self, block_height: u32) -> Result<()> {
        // Ensure the block exists.
        if self.block_heights.contains_key(&block_height)? {
            Err(anyhow!("Block {} does not exists in storage", block_height))
        } else {
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
            self.block_heights.remove(&block_height)?;
            // Remove the block header.
            self.block_headers.remove(&block_hash)?;
            // Remove the block transactions.
            self.block_transactions.remove(&block_hash)?;
            // Remove the transactions.
            for transaction_ids in transaction_ids.iter() {
                self.transactions.remove_transaction(transaction_ids)?;
            }

            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
struct TransactionState<N: Network> {
    transactions: DataMap<N::TransactionID, (N::BlockHash, u16, N::LedgerRoot, Vec<N::TransitionID>)>,
    transitions: DataMap<N::TransitionID, (N::TransactionID, u8, Transition<N>)>,
    serial_numbers: DataMap<N::SerialNumber, N::TransitionID>,
    commitments: DataMap<N::Commitment, N::TransitionID>,
    ciphertext_ids: DataMap<N::CiphertextID, N::TransitionID>,
    // events: DataMap<N::TransactionID, Vec<Event<N>>>,
}

impl<N: Network> TransactionState<N> {
    /// Initializes a new instance of `TransactionState`.
    fn open<S: Storage>(storage: S) -> Result<Self> {
        Ok(Self {
            transactions: storage.open_map("transactions")?,
            transitions: storage.open_map("transitions")?,
            serial_numbers: storage.open_map("serial_numbers")?,
            commitments: storage.open_map("commitments")?,
            ciphertext_ids: storage.open_map("ciphertext_ids")?,
            // events
        })
    }

    /// Returns `true` if the given transaction ID exists in storage.
    fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        self.transactions.contains_key(transaction_id)
    }

    /// Returns `true` if the given serial number exists in storage.
    fn contains_serial_number(&self, serial_number: &N::SerialNumber) -> Result<bool> {
        self.serial_numbers.contains_key(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub(crate) fn contains_commitment(&self, commitment: &N::Commitment) -> Result<bool> {
        self.commitments.contains_key(commitment)
    }

    /// Returns `true` if the given ciphertext ID exists in storage.
    pub(crate) fn contains_ciphertext_id(&self, ciphertext_id: &N::CiphertextID) -> Result<bool> {
        self.ciphertext_ids.contains_key(ciphertext_id)
    }

    /// Returns the record ciphertext for a given ciphertext ID.
    pub(crate) fn get_ciphertext(&self, ciphertext_id: &N::CiphertextID) -> Result<RecordCiphertext<N>> {
        // Retrieve the transition ID.
        let transition_id = match self.ciphertext_ids.get(ciphertext_id)? {
            Some(transition_id) => transition_id,
            None => return Err(anyhow!("Ciphertext {} does not exist in storage", ciphertext_id)),
        };

        // Retrieve the transition.
        let transition = match self.transitions.get(&transition_id)? {
            Some((_, _, transition)) => transition,
            None => return Err(anyhow!("Transition {} does not exist in storage", transition_id)),
        };

        // Retrieve the ciphertext.
        for (i, candidate_ciphertext_id) in transition.to_ciphertext_ids().enumerate() {
            if candidate_ciphertext_id? == *ciphertext_id {
                return Ok(transition.ciphertexts()[i].clone());
            }
        }

        Err(anyhow!("Ciphertext {} is missing in storage", ciphertext_id))
    }

    /// Returns the transition for a given transition ID.
    pub(crate) fn get_transition(&self, transition_id: &N::TransitionID) -> Result<Transition<N>> {
        match self.transitions.get(transition_id)? {
            Some((_, _, transition)) => Ok(transition),
            None => return Err(anyhow!("Transition {} does not exist in storage", transition_id)),
        }
    }

    /// Returns the transaction for a given transaction ID.
    pub(crate) fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        // Retrieve the transition IDs.
        let (ledger_root, transition_ids) = match self.transactions.get(transaction_id)? {
            Some((_, _, ledger_root, transition_ids)) => (ledger_root, transition_ids),
            None => return Err(anyhow!("Transaction {} does not exist in storage", transaction_id)),
        };

        // Retrieve the transitions.
        let mut transitions = Vec::with_capacity(transition_ids.len());
        for transition_id in transition_ids.iter() {
            match self.transitions.get(transition_id)? {
                Some((_, _, transition)) => transitions.push(transition),
                None => return Err(anyhow!("Transition {} missing in storage", transition_id)),
            };
        }

        Transaction::from(*N::inner_circuit_id(), ledger_root, transitions, vec![])
    }

    /// Adds the given transaction to storage.
    pub(crate) fn add_transaction(&self, transaction: &Transaction<N>, block_hash: N::BlockHash, transaction_index: u16) -> Result<()> {
        // Ensure the transaction does not exist.
        let transaction_id = transaction.transaction_id();
        if self.transactions.contains_key(&transaction_id)? {
            Err(anyhow!("Transaction {} already exists in storage", transaction_id))
        } else {
            let transition_ids = transaction.transition_ids();
            let transitions = transaction.transitions();
            let ledger_root = transaction.ledger_root();

            // Insert the transaction ID.
            self.transactions
                .insert(&transaction_id, &(block_hash, transaction_index, ledger_root, transition_ids))?;

            for (i, transition) in transitions.iter().enumerate() {
                let transition_id = transition.transition_id();

                // Insert the transition.
                self.transitions
                    .insert(&transition_id, &(transaction_id, i as u8, transition.clone()))?;

                // Insert the serial numbers.
                for serial_number in transition.serial_numbers() {
                    self.serial_numbers.insert(serial_number, &transition_id)?;
                }
                // Insert the commitments.
                for commitment in transition.commitments() {
                    self.commitments.insert(commitment, &transition_id)?;
                }
                // Insert the ciphertext IDs.
                for ciphertext_id in transition.to_ciphertext_ids() {
                    self.ciphertext_ids.insert(&ciphertext_id?, &transition_id)?;
                }
            }

            Ok(())
        }
    }

    /// Removes the given transaction ID from storage.
    pub(crate) fn remove_transaction(&self, transaction_id: &N::TransactionID) -> Result<()> {
        // Ensure the transaction exists.
        if !self.transactions.contains_key(&transaction_id)? {
            Err(anyhow!("Transaction {} does not exist in storage", transaction_id))
        } else {
            // Retrieve the transition IDs from the transaction.
            let transition_ids = match self.transactions.get(&transaction_id)? {
                Some((_, _, _, transition_ids)) => transition_ids,
                None => return Err(anyhow!("Transaction {} missing from transactions map", transaction_id)),
            };

            // Remove the transaction entry.
            self.transactions.remove(&transaction_id)?;

            for (_, transition_id) in transition_ids.iter().enumerate() {
                // Retrieve the transition from the transition ID.
                let transition = match self.transitions.get(&transition_id)? {
                    Some((_, _, transition)) => transition,
                    None => return Err(anyhow!("Transition {} missing from transitions map", transition_id)),
                };

                // Remove the transition.
                self.transitions.remove(&transition_id)?;

                // Remove the serial numbers.
                for serial_number in transition.serial_numbers() {
                    self.serial_numbers.remove(serial_number)?;
                }
                // Remove the commitments.
                for commitment in transition.commitments() {
                    self.commitments.remove(commitment)?;
                }
                // Remove the ciphertext IDs.
                for ciphertext_id in transition.to_ciphertext_ids() {
                    self.ciphertext_ids.remove(&ciphertext_id?)?;
                }
            }

            Ok(())
        }
    }
}
