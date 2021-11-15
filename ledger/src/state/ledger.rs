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

use crate::{
    helpers::BlockLocators,
    storage::{DataMap, Map, Storage},
};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use circular_queue::CircularQueue;
use itertools::Itertools;
use parking_lot::RwLock;
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread,
};

/// The number of seconds in two hours.
const TWO_HOURS_UNIX: i64 = 7200;

/// The maximum number of linear block locators.
pub const MAXIMUM_LINEAR_BLOCK_LOCATORS: u32 = 256;
/// The maximum number of quadratic block locators.
pub const MAXIMUM_QUADRATIC_BLOCK_LOCATORS: u32 = 64;
/// The total maximum number of block locators.
pub const MAXIMUM_BLOCK_LOCATORS: u32 = MAXIMUM_LINEAR_BLOCK_LOCATORS.saturating_add(MAXIMUM_QUADRATIC_BLOCK_LOCATORS);

///
/// A helper struct containing transaction metadata.
///
/// *Attention*: This data structure is intended for usage in storage only.
/// Modifications to its layout will impact how metadata is represented in storage.
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Metadata<N: Network> {
    block_height: u32,
    block_hash: N::BlockHash,
    block_timestamp: i64,
    transaction_index: u16,
}

impl<N: Network> Metadata<N> {
    /// Initializes a new instance of `Metadata`.
    pub fn new(block_height: u32, block_hash: N::BlockHash, block_timestamp: i64, transaction_index: u16) -> Self {
        Self {
            block_height,
            block_hash,
            block_timestamp,
            transaction_index,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LedgerState<N: Network> {
    /// The current ledger tree of block hashes.
    ledger_tree: Arc<RwLock<LedgerTree<N>>>,
    /// The latest block of the ledger.
    latest_block: Arc<RwLock<Block<N>>>,
    /// The latest block hashes of the ledger.
    latest_block_hashes: Arc<RwLock<CircularQueue<N::BlockHash>>>,
    /// The latest block headers of the ledger.
    latest_block_headers: Arc<RwLock<CircularQueue<BlockHeader<N>>>>,
    /// The block locators from the latest block of the ledger.
    latest_block_locators: Arc<RwLock<BlockLocators<N>>>,
    /// The ledger root corresponding to each block height.
    ledger_roots: DataMap<N::LedgerRoot, u32>,
    /// The blocks of the ledger in storage.
    blocks: BlockState<N>,
    /// The indicator bit and tracker for a ledger in read-only mode.
    read_only: (bool, Arc<AtomicU32>),
}

impl<N: Network> LedgerState<N> {
    /// Initializes a new instance of `LedgerState`.
    pub fn open<S: Storage, P: AsRef<Path>>(path: P, is_read_only: bool) -> Result<Self> {
        // Open storage.
        let context = N::NETWORK_ID;
        let storage = S::open(path, context, is_read_only)?;

        // Retrieve the genesis block.
        let genesis = N::genesis_block();
        // Initialize the ledger.
        let mut ledger = Self {
            ledger_tree: Arc::new(RwLock::new(LedgerTree::<N>::new()?)),
            latest_block: Arc::new(RwLock::new(genesis.clone())),
            latest_block_hashes: Arc::new(RwLock::new(CircularQueue::with_capacity(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize))),
            latest_block_headers: Arc::new(RwLock::new(CircularQueue::with_capacity(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize))),
            latest_block_locators: Default::default(),
            ledger_roots: storage.open_map("ledger_roots")?,
            blocks: BlockState::open(storage)?,
            read_only: (is_read_only, Arc::new(AtomicU32::new(genesis.height()))),
        };

        // Determine the latest block height.
        let latest_block_height = match (ledger.ledger_roots.values().max(), ledger.blocks.block_heights.keys().max()) {
            (Some(latest_block_height_0), Some(latest_block_height_1)) => match latest_block_height_0 == latest_block_height_1 {
                true => latest_block_height_0,
                false => return Err(anyhow!("Ledger storage state is incorrect")),
            },
            (None, None) => 0u32,
            _ => return Err(anyhow!("Ledger storage state is inconsistent")),
        };

        // If this is new storage, initialize it with the genesis block.
        if latest_block_height == 0u32 && !ledger.blocks.contains_block_height(0u32)? {
            ledger.ledger_roots.insert(&genesis.previous_ledger_root(), &genesis.height())?;
            ledger.blocks.add_block(genesis)?;
        }

        // Retrieve each block from genesis to validate state.
        for block_height in 0..=latest_block_height {
            // Validate the ledger root every 250 blocks.
            if block_height % 250 == 0 || block_height == latest_block_height {
                if !ledger.is_read_only() {
                    debug!("Validating the ledger root up to block {}", block_height);
                }

                // Ensure the ledger roots match their expected block heights.
                let expected_ledger_root = ledger.get_previous_ledger_root(block_height)?;
                match ledger.ledger_roots.get(&expected_ledger_root)? {
                    Some(height) => {
                        if block_height != height {
                            return Err(anyhow!("Ledger expected block {}, found block {}", block_height, height));
                        }
                    }
                    None => return Err(anyhow!("Ledger is missing ledger root for block {}", block_height)),
                }

                // Ensure the ledger tree matches the state of ledger roots.
                if expected_ledger_root != ledger.ledger_tree.read().root() {
                    return Err(anyhow!("Ledger has incorrect ledger tree state at block {}", block_height));
                }
            }

            // Add the block hash to the ledger tree.
            ledger.ledger_tree.write().add(&ledger.get_block_hash(block_height)?)?;
        }

        // Update the latest ledger state.
        *ledger.latest_block.write() = ledger.get_block(latest_block_height)?;
        ledger.regenerate_latest_ledger_state()?;

        // Validate the ledger root one final time.
        let latest_ledger_root = ledger.ledger_tree.read().root();
        ledger.regenerate_ledger_tree()?;
        assert_eq!(ledger.ledger_tree.read().root(), latest_ledger_root);

        // If the ledger is in read-only mode, proceed to start a process to keep the reader in sync.
        if ledger.is_read_only() {
            debug!("Loading ledger in read-only mode");
            let mut ledger = ledger.clone();
            thread::spawn(move || {
                let last_seen_block_height = ledger.read_only.1.clone();
                ledger.read_only.1.store(latest_block_height, Ordering::SeqCst);

                loop {
                    // Refresh the ledger storage state.
                    if ledger.ledger_roots.refresh() {
                        // After catching up the reader, determine the latest block height.
                        if let Some(latest_block_height) = ledger.blocks.block_heights.keys().max() {
                            let current_block_height = last_seen_block_height.load(Ordering::SeqCst);
                            trace!(
                                "[Read-Only] Updating ledger state from block {} to {}",
                                current_block_height,
                                latest_block_height
                            );

                            // Update the latest block.
                            match ledger.get_block(latest_block_height) {
                                Ok(block) => *ledger.latest_block.write() = block,
                                Err(error) => warn!("[Read-Only] {}", error),
                            };
                            // Regenerate the ledger tree.
                            if let Err(error) = ledger.regenerate_ledger_tree() {
                                warn!("[Read-Only] {}", error);
                            };
                            // Regenerate the latest ledger state.
                            if let Err(error) = ledger.regenerate_latest_ledger_state() {
                                warn!("[Read-Only] {}", error);
                            };
                            // Update the last seen block height.
                            last_seen_block_height.store(latest_block_height, Ordering::SeqCst);
                        }
                    }
                    thread::sleep(std::time::Duration::from_secs(8));
                }
            });
        }

        // let value = storage.export()?;
        // println!("{}", value);
        // let storage_2 = S::open(".ledger_2", context)?;
        // storage_2.import(value)?;

        debug!("Loaded ledger from block {}", ledger.latest_block_height());
        Ok(ledger)
    }

    /// Returns `true` if the ledger is in read-only mode.
    pub fn is_read_only(&self) -> bool {
        self.read_only.0
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Block<N> {
        self.latest_block.read().clone()
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.latest_block.read().height()
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.latest_block.read().hash()
    }

    /// Returns the latest block timestamp.
    pub fn latest_block_timestamp(&self) -> i64 {
        self.latest_block.read().timestamp()
    }

    /// Returns the latest block difficulty target.
    pub fn latest_block_difficulty_target(&self) -> u64 {
        self.latest_block.read().difficulty_target()
    }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> BlockHeader<N> {
        self.latest_block.read().header().clone()
    }

    /// Returns the transactions from the latest block.
    pub fn latest_block_transactions(&self) -> Transactions<N> {
        self.latest_block.read().transactions().clone()
    }

    /// Returns the latest block locators.
    pub fn latest_block_locators(&self) -> BlockLocators<N> {
        self.latest_block_locators.read().clone()
    }

    /// Returns the latest ledger root.
    pub fn latest_ledger_root(&self) -> N::LedgerRoot {
        self.ledger_tree.read().root()
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

    /// Returns the transaction metadata for a given transaction ID.
    pub fn get_transaction_metadata(&self, transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        self.blocks.get_transaction_metadata(transaction_id)
    }

    /// Returns the block height for the given block hash.
    pub fn get_block_height(&self, block_hash: &N::BlockHash) -> Result<u32> {
        self.blocks.get_block_height(block_hash)
    }

    /// Returns the block hash for the given block height.
    pub fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.blocks.get_block_hash(block_height)
    }

    /// Returns the block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        self.blocks.get_block_hashes(start_block_height, end_block_height)
    }

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.blocks.get_previous_block_hash(block_height)
    }

    /// Returns the block header for the given block height.
    pub fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        self.blocks.get_block_header(block_height)
    }

    /// Returns the block headers from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_headers(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<BlockHeader<N>>> {
        self.blocks.get_block_headers(start_block_height, end_block_height)
    }

    /// Returns the transactions from the block of the given block height.
    pub fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        self.blocks.get_block_transactions(block_height)
    }

    /// Returns the block for a given block height.
    pub fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        self.blocks.get_block(block_height)
    }

    /// Returns the blocks from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        self.blocks.get_blocks(start_block_height, end_block_height)
    }

    /// Returns the ledger root in the block header of the given block height.
    pub fn get_previous_ledger_root(&self, block_height: u32) -> Result<N::LedgerRoot> {
        self.blocks.get_previous_ledger_root(block_height)
    }

    /// Returns the block locators of the current ledger, from the given block height.
    pub fn get_block_locators(&self, block_height: u32) -> Result<BlockLocators<N>> {
        // Initialize the current block height that a block locator is obtained from.
        let mut block_locator_height = block_height;

        // Determine the number of latest block headers to include as block locators (linear).
        let num_block_headers = std::cmp::min(MAXIMUM_LINEAR_BLOCK_LOCATORS, block_locator_height);

        // Acquire the read lock for the latest block hashes and block headers.
        let latest_block_hashes = self.latest_block_hashes.read();
        let latest_block_headers = self.latest_block_headers.read();

        // Construct the list of block locator headers.
        let block_hashes = latest_block_hashes.iter().cloned();
        let block_headers = latest_block_headers.iter().cloned();
        let block_locator_headers = block_hashes
            .zip_eq(block_headers)
            .take(num_block_headers as usize)
            .map(|(hash, header)| (header.height(), (hash, Some(header))));

        // Decrement the block locator height by the number of block headers.
        block_locator_height -= num_block_headers;

        // Return the block locators if the locator has run out of blocks.
        if block_locator_height == 0 {
            // Initialize the list of block locators.
            let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<BlockHeader<N>>)> = block_locator_headers.collect();
            // Add the genesis locator.
            block_locators.insert(0, (self.get_block_hash(0)?, None));

            return Ok(BlockLocators::<N>::from(block_locators));
        }

        // Determine the number of latest block hashes to include as block locators (power of two).
        let num_block_hashes = std::cmp::min(MAXIMUM_QUADRATIC_BLOCK_LOCATORS, block_locator_height);

        // Initialize list of block locator hashes.
        let mut block_locator_hashes = Vec::with_capacity(num_block_hashes as usize);
        // Add the block locator hashes.
        while block_locator_height > 0 && block_locator_hashes.len() < num_block_hashes as usize {
            block_locator_hashes.push((block_locator_height, (self.get_block_hash(block_locator_height)?, None)));

            // Decrement the block locator height by a power of two.
            block_locator_height /= 2;
        }

        // Initialize the list of block locators.
        let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<BlockHeader<N>>)> =
            block_locator_headers.chain(block_locator_hashes).collect();
        // Add the genesis locator.
        block_locators.insert(0, (self.get_block_hash(0)?, None));

        Ok(BlockLocators::<N>::from(block_locators))
    }

    /// Check that the block locators are well formed.
    pub fn check_block_locators(&self, block_locators: &BlockLocators<N>) -> Result<bool> {
        // Check that the number of block_locators is less than the total MAXIMUM_BLOCK_LOCATORS.
        if block_locators.len() > MAXIMUM_BLOCK_LOCATORS as usize {
            return Ok(false);
        }

        let block_locators = &**block_locators;

        // Ensure the genesis block locator exists and is well-formed.
        let (expected_genesis_block_hash, expected_genesis_header) = match block_locators.get(&0) {
            Some((expected_genesis_block_hash, expected_genesis_header)) => (expected_genesis_block_hash, expected_genesis_header),
            None => return Ok(false),
        };
        if expected_genesis_block_hash != &N::genesis_block().hash() || expected_genesis_header.is_some() {
            return Ok(false);
        }

        // // Get the remaining block locators (excluding the genesis block).
        // let remaining_block_locators = &block_locators[..block_locators.len() - 1];
        // let num_block_headers = std::cmp::min(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize, remaining_block_locators.len());
        //
        // // Check that the block headers are formed correctly (linear).
        // // let mut last_block_height = remaining_block_locators[0].0 + 1;
        // for (_block_height, _block_hash, block_header) in &remaining_block_locators[..num_block_headers] {
        //     // // Check that the block height is decrementing.
        //     // match last_block_height == *block_height + 1 {
        //     //     true => last_block_height = *block_height,
        //     //     false => return Ok(false)
        //     // }
        //
        //     // Check that the block header is present.
        //     let _block_header = match block_header {
        //         Some(header) => header,
        //         None => return Ok(false),
        //     };
        //
        //     // // Check that the expected block hash is correct.
        //     // if let Ok(expected_block_hash) = self.get_block_hash(*block_height) {
        //     //     if &expected_block_hash != block_hash {
        //     //         return Ok(false);
        //     //     }
        //     // }
        //     //
        //     // // Check that the expected block headers is correct.
        //     // if let Ok(expected_block_header) = self.get_block_header(*block_height) {
        //     //     if &expected_block_header != block_header {
        //     //         return Ok(false);
        //     //     }
        //     // }
        // }

        // // Check that the block hashes are formed correctly (power of two).
        // if block_locators.len() > MAXIMUM_LINEAR_BLOCK_LOCATORS as usize {
        //     let mut previous_block_height = u32::MAX;
        //
        //     for (block_height, _block_hash, block_header) in &block_locators[num_block_headers..] {
        //         // Check that the block heights increment by a power of two.
        //         if previous_block_height != u32::MAX && previous_block_height / 2 != *block_height {
        //             return Ok(false);
        //         }
        //
        //         // Check that there is no block header.
        //         if block_header.is_some() {
        //             return Ok(false);
        //         }
        //
        //         // // Check that the expected block hash is correct.
        //         // if let Ok(expected_block_hash) = self.get_block_hash(*block_height) {
        //         //     if &expected_block_hash != block_hash {
        //         //         return Ok(false);
        //         //     }
        //         // }
        //
        //         previous_block_height = *block_height;
        //     }
        // }

        Ok(true)
    }

    /// Mines a new block using the latest state of the given ledger.
    pub fn mine_next_block<R: Rng + CryptoRng>(
        &self,
        recipient: Address<N>,
        transactions: &[Transaction<N>],
        terminator: &AtomicBool,
        rng: &mut R,
    ) -> Result<Block<N>> {
        // Prepare the new block.
        let previous_block_hash = self.latest_block_hash();
        let block_height = self.latest_block_height() + 1;

        // Compute the block difficulty target.
        let previous_timestamp = self.latest_block_timestamp();
        let previous_difficulty_target = self.latest_block_difficulty_target();
        let block_timestamp = chrono::Utc::now().timestamp();
        let difficulty_target = Blocks::<N>::compute_difficulty_target(previous_timestamp, previous_difficulty_target, block_timestamp);

        // Construct the ledger root.
        let ledger_root = self.latest_ledger_root();

        // Craft a coinbase transaction.
        let amount = Block::<N>::block_reward(block_height);
        let coinbase_transaction = Transaction::<N>::new_coinbase(recipient, amount, rng)?;

        // Construct the new block transactions.
        let transactions = Transactions::from(&[&[coinbase_transaction], transactions].concat())?;

        // Mine the next block.
        match Block::mine(
            previous_block_hash,
            block_height,
            block_timestamp,
            difficulty_target,
            ledger_root,
            transactions,
            terminator,
            rng,
        ) {
            Ok(block) => Ok(block),
            Err(error) => Err(anyhow!("Unable to mine the next block: {}", error)),
        }
    }

    /// Adds the given block as the next block in the ledger to storage.
    pub fn add_next_block(&mut self, block: &Block<N>) -> Result<()> {
        // If the storage is in read-only mode, this method cannot be called.
        if self.is_read_only() {
            return Err(anyhow!("Ledger is in read-only mode"));
        }

        // Ensure the block itself is valid.
        if !block.is_valid() {
            return Err(anyhow!("Block {} is invalid", block.height()));
        }

        // Retrieve the current block.
        let current_block = self.latest_block();

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
        if block.previous_block_hash() != current_block.hash() {
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
        if self.contains_block_hash(&block.hash())? {
            return Err(anyhow!("Block {} has a repeat block hash in the canon chain", block_height));
        }

        // Ensure the ledger root in the block matches the current ledger root.
        if block.previous_ledger_root() != self.latest_ledger_root() {
            return Err(anyhow!("Block {} declares an incorrect ledger root", block_height));
        }

        // Ensure the canon chain does not already contain the given serial numbers.
        for serial_number in block.serial_numbers() {
            if self.contains_serial_number(serial_number)? {
                return Err(anyhow!("Serial number {} already exists in the ledger", serial_number));
            }
        }

        // Ensure the canon chain does not already contain the given commitments.
        for commitment in block.commitments() {
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
        self.ledger_tree.write().add(&block.hash())?;
        self.ledger_roots.insert(&block.previous_ledger_root(), &block.height())?;
        self.latest_block_hashes.write().push(block.hash());
        self.latest_block_headers.write().push(block.header().clone());
        *self.latest_block_locators.write() = self.get_block_locators(block.height())?;
        *self.latest_block.write() = block.clone();
        Ok(())
    }

    /// Reverts the ledger state back to the given block height, returning the removed blocks on success.
    pub fn revert_to_block_height(&mut self, block_height: u32) -> Result<Vec<Block<N>>> {
        // If the storage is in read-only mode, this method cannot be called.
        if self.is_read_only() {
            return Err(anyhow!("Ledger is in read-only mode"));
        }

        // Determine the number of blocks to remove.
        let latest_block_height = self.latest_block_height();
        let number_of_blocks = latest_block_height.saturating_sub(block_height);

        // Ensure the reverted block height is within a permitted range and well-formed.
        if block_height >= latest_block_height || number_of_blocks > N::ALEO_MAXIMUM_FORK_DEPTH || self.get_block(block_height).is_err() {
            return Err(anyhow!("Attempted to return to block height {}, which is invalid", block_height));
        }

        // Initialize a list of the removed blocks.
        let mut blocks = Vec::with_capacity(number_of_blocks as usize);
        {
            // Acquire the write lock on the latest block.
            let mut latest_block = self.latest_block.write();

            while latest_block.height() > block_height {
                // Update the internal state of the ledger, except for the ledger tree.
                self.blocks.remove_block(latest_block.height())?;
                self.ledger_roots.remove(&latest_block.previous_ledger_root())?;

                // Append this block to the final output.
                blocks.push(latest_block.clone());

                *latest_block = match latest_block.height() == 0 {
                    true => N::genesis_block().clone(),
                    false => self.get_block(latest_block.height().saturating_sub(1))?,
                };
            }
        }

        // Regenerate the latest ledger state.
        self.regenerate_latest_ledger_state()?;
        // Regenerate the ledger tree.
        self.regenerate_ledger_tree()?;

        // Reverse the order of the blocks, so they are in increasing order (i.e. 1, 2, 3...).
        blocks.reverse();
        // Return the removed blocks.
        Ok(blocks)
    }

    ///
    /// Returns a ledger proof for the given commitment.
    ///
    pub fn get_ledger_inclusion_proof(&self, commitment: N::Commitment) -> Result<LedgerProof<N>> {
        // TODO (raychu86): Add getter functions.
        let commitment_transition_id = match self.blocks.transactions.commitments.get(&commitment)? {
            Some(transition_id) => transition_id,
            None => return Err(anyhow!("commitment {} missing from commitments map", commitment)),
        };

        let transaction_id = match self.blocks.transactions.transitions.get(&commitment_transition_id)? {
            Some((transaction_id, _, _)) => transaction_id,
            None => return Err(anyhow!("transition id {} missing from transactions map", commitment_transition_id)),
        };

        let transaction = self.get_transaction(&transaction_id)?;

        let block_hash = match self.blocks.transactions.transactions.get(&transaction_id)? {
            Some((_, _, metadata)) => metadata.block_hash,
            None => return Err(anyhow!("transaction id {} missing from transactions map", transaction_id)),
        };

        let block_header = match self.blocks.block_headers.get(&block_hash)? {
            Some(block_header) => block_header,
            None => return Err(anyhow!("Block {} missing from block headers map", block_hash)),
        };

        // Generate the local proof for the commitment.
        let local_proof = transaction.to_local_proof(commitment)?;

        let transaction_id = local_proof.transaction_id();
        let transactions = self.get_block_transactions(block_header.height())?;

        // Compute the transactions inclusion proof.
        let transactions_inclusion_proof = {
            // TODO (howardwu): Optimize this operation.
            let index = transactions
                .transaction_ids()
                .enumerate()
                .filter_map(|(index, id)| match id == transaction_id {
                    true => Some(index),
                    false => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(1, index.len()); // TODO (howardwu): Clean this up with a proper error handler.
            transactions.to_transactions_inclusion_proof(index[0], transaction_id)?
        };

        // Compute the block header inclusion proof.
        let transactions_root = transactions.transactions_root();
        let block_header_inclusion_proof = block_header.to_header_inclusion_proof(1, transactions_root)?;
        let block_header_root = block_header.to_header_root()?;

        // Determine the previous block hash.
        let previous_block_hash = self.get_previous_block_hash(self.get_block_height(&block_hash)?)?;

        // Generate the record proof.
        let record_proof = RecordProof::new(
            block_hash,
            previous_block_hash,
            block_header_root,
            block_header_inclusion_proof,
            transactions_root,
            transactions_inclusion_proof,
            local_proof,
        )?;

        // Generate the ledger root inclusion proof.
        let ledger_root = self.ledger_tree.read().root();
        let ledger_root_inclusion_proof = self.ledger_tree.read().to_ledger_inclusion_proof(&block_hash)?;

        LedgerProof::new(ledger_root, ledger_root_inclusion_proof, record_proof)
    }

    /// Updates the latest block hashes and block headers.
    fn regenerate_latest_ledger_state(&mut self) -> Result<()> {
        // Compute the start block height and end block height (inclusive).
        let end_block_height = self.latest_block_height();
        let start_block_height = end_block_height.saturating_sub(MAXIMUM_LINEAR_BLOCK_LOCATORS - 1);

        // Retrieve the latest block hashes and block headers.
        let block_hashes = self.get_block_hashes(start_block_height, end_block_height)?;
        let block_headers = self.get_block_headers(start_block_height, end_block_height)?;
        assert_eq!(block_hashes.len(), block_headers.len());

        {
            // Acquire the write lock for the latest block hashes and block headers.
            let mut latest_block_hashes = self.latest_block_hashes.write();
            let mut latest_block_headers = self.latest_block_headers.write();

            // Upon success, clear the latest ledger state.
            latest_block_hashes.clear();
            latest_block_headers.clear();

            // Add the latest block hashes and block headers.
            for (block_hash, block_header) in block_hashes.into_iter().zip_eq(block_headers) {
                latest_block_hashes.push(block_hash);
                latest_block_headers.push(block_header);
            }
        }

        *self.latest_block_locators.write() = self.get_block_locators(end_block_height)?;

        Ok(())
    }

    // TODO (raychu86): Make this more efficient.
    /// Updates the ledger tree.
    fn regenerate_ledger_tree(&mut self) -> Result<()> {
        // Acquire the ledger tree write lock.
        let mut ledger_tree = self.ledger_tree.write();

        // Retrieve all of the block hashes.
        let mut block_hashes = Vec::with_capacity(self.latest_block_height() as usize);
        for height in 0..=self.latest_block_height() {
            block_hashes.push(self.get_block_hash(height)?);
        }

        // Add the block hashes to create the new ledger tree.
        let mut new_ledger_tree = LedgerTree::<N>::new()?;
        new_ledger_tree.add_all(&block_hashes)?;

        // Update the current ledger tree with the current state.
        *ledger_tree = new_ledger_tree;

        Ok(())
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

    /// Returns the transaction metadata for a given transaction ID.
    fn get_transaction_metadata(&self, transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        self.transactions.get_transaction_metadata(transaction_id)
    }

    /// Returns the block height for the given block hash.
    fn get_block_height(&self, block_hash: &N::BlockHash) -> Result<u32> {
        match self.block_headers.get(block_hash)? {
            Some(block_header) => Ok(block_header.height()),
            None => return Err(anyhow!("Block {} missing from block headers map", block_hash)),
        }
    }

    /// Returns the block hash for the given block height.
    fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.get_previous_block_hash(block_height + 1)
    }

    /// Returns the block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        // Ensure the starting block height is less than the ending block height.
        if start_block_height > end_block_height {
            return Err(anyhow!("Invalid starting and ending block heights"));
        }

        (start_block_height..=end_block_height)
            .into_iter()
            .map(|height| self.get_block_hash(height))
            .collect()
    }

    /// Returns the previous block hash for the given block height.
    fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        match block_height == 0 {
            true => Ok(N::genesis_block().previous_block_hash()),
            false => match self.block_heights.get(&(block_height - 1))? {
                Some(block_hash) => Ok(block_hash),
                None => return Err(anyhow!("Block {} missing in block heights map", block_height - 1)),
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

    /// Returns the block headers from the given `start_block_height` to `end_block_height` (inclusive).
    fn get_block_headers(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<BlockHeader<N>>> {
        // Ensure the starting block height is less than the ending block height.
        if start_block_height > end_block_height {
            return Err(anyhow!("Invalid starting and ending block heights"));
        }

        (start_block_height..=end_block_height)
            .into_iter()
            .map(|height| self.get_block_header(height))
            .collect()
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

        Ok(Block::from(previous_block_hash, block_header, transactions)?)
    }

    /// Returns the blocks from the given `start_block_height` to `end_block_height` (inclusive).
    fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        // Ensure the starting block height is less than the ending block height.
        if start_block_height > end_block_height {
            return Err(anyhow!("Invalid starting and ending block heights"));
        }

        (start_block_height..=end_block_height)
            .into_iter()
            .map(|height| self.get_block(height))
            .collect()
    }

    /// Returns the ledger root in the block header of the given block height.
    fn get_previous_ledger_root(&self, block_height: u32) -> Result<N::LedgerRoot> {
        // Retrieve the block header.
        let block_header = self.get_block_header(block_height)?;
        // Return the ledger root in the block header.
        Ok(block_header.previous_ledger_root())
    }

    /// Adds the given block to storage.
    fn add_block(&self, block: &Block<N>) -> Result<()> {
        // Ensure the block does not exist.
        let block_height = block.height();
        if self.block_heights.contains_key(&block_height)? {
            Err(anyhow!("Block {} already exists in storage", block_height))
        } else {
            let block_hash = block.hash();
            let block_header = block.header();
            let transactions = block.transactions();
            let transaction_ids = transactions.transaction_ids().collect::<Vec<_>>();

            // Insert the block height.
            self.block_heights.insert(&block_height, &block_hash)?;
            // Insert the block header.
            self.block_headers.insert(&block_hash, block_header)?;
            // Insert the block transactions.
            self.block_transactions.insert(&block_hash, &transaction_ids)?;
            // Insert the transactions.
            for (index, transaction) in transactions.iter().enumerate() {
                let metadata = Metadata::<N>::new(block_height, block_hash, block.timestamp(), index as u16);
                self.transactions.add_transaction(transaction, metadata)?;
            }

            Ok(())
        }
    }

    /// Removes the given block height from storage.
    fn remove_block(&self, block_height: u32) -> Result<()> {
        // Ensure the block height is not the genesis block.
        if block_height == 0 {
            Err(anyhow!("Block {} cannot be removed from storage", block_height))
        }
        // Ensure the block at the given block height exists.
        else if !self.block_heights.contains_key(&block_height)? {
            Err(anyhow!("Block {} does not exist in storage", block_height))
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
#[allow(clippy::type_complexity)]
struct TransactionState<N: Network> {
    transactions: DataMap<N::TransactionID, (N::LedgerRoot, Vec<N::TransitionID>, Metadata<N>)>,
    transitions: DataMap<N::TransitionID, (N::TransactionID, u8, Transition<N>)>,
    serial_numbers: DataMap<N::SerialNumber, N::TransitionID>,
    commitments: DataMap<N::Commitment, N::TransitionID>,
    ciphertext_ids: DataMap<N::CiphertextID, N::TransitionID>,
    events: DataMap<N::TransactionID, Vec<Event<N>>>,
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
            events: storage.open_map("events")?,
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
    fn contains_commitment(&self, commitment: &N::Commitment) -> Result<bool> {
        self.commitments.contains_key(commitment)
    }

    /// Returns `true` if the given ciphertext ID exists in storage.
    fn contains_ciphertext_id(&self, ciphertext_id: &N::CiphertextID) -> Result<bool> {
        self.ciphertext_ids.contains_key(ciphertext_id)
    }

    /// Returns the record ciphertext for a given ciphertext ID.
    fn get_ciphertext(&self, ciphertext_id: &N::CiphertextID) -> Result<RecordCiphertext<N>> {
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
        for (candidate_ciphertext_id, candidate_ciphertext) in transition.to_ciphertext_ids().zip_eq(transition.ciphertexts()) {
            if candidate_ciphertext_id? == *ciphertext_id {
                return Ok(candidate_ciphertext.clone());
            }
        }

        Err(anyhow!("Ciphertext {} is missing in storage", ciphertext_id))
    }

    /// Returns the transition for a given transition ID.
    fn get_transition(&self, transition_id: &N::TransitionID) -> Result<Transition<N>> {
        match self.transitions.get(transition_id)? {
            Some((_, _, transition)) => Ok(transition),
            None => return Err(anyhow!("Transition {} does not exist in storage", transition_id)),
        }
    }

    /// Returns the transaction for a given transaction ID.
    fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        // Retrieve the transition IDs.
        let (ledger_root, transition_ids) = match self.transactions.get(transaction_id)? {
            Some((ledger_root, transition_ids, _)) => (ledger_root, transition_ids),
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

    /// Returns the transaction metadata for a given transaction ID.
    fn get_transaction_metadata(&self, transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        // Retrieve the metadata from the transactions map.
        match self.transactions.get(transaction_id)? {
            Some((_, _, metadata)) => Ok(metadata),
            None => Err(anyhow!("Transaction {} does not exist in storage", transaction_id)),
        }
    }

    /// Adds the given transaction to storage.
    fn add_transaction(&self, transaction: &Transaction<N>, metadata: Metadata<N>) -> Result<()> {
        // Ensure the transaction does not exist.
        let transaction_id = transaction.transaction_id();
        if self.transactions.contains_key(&transaction_id)? {
            Err(anyhow!("Transaction {} already exists in storage", transaction_id))
        } else {
            let transition_ids = transaction.transition_ids().collect();
            let transitions = transaction.transitions();
            let ledger_root = transaction.ledger_root();

            // Insert the transaction ID.
            self.transactions
                .insert(&transaction_id, &(ledger_root, transition_ids, metadata))?;

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
                    self.ciphertext_ids.insert(&*ciphertext_id?, &transition_id)?;
                }
            }

            // Insert the transaction events.
            self.events.insert(&transaction_id, transaction.events())?;

            Ok(())
        }
    }

    /// Removes the given transaction ID from storage.
    fn remove_transaction(&self, transaction_id: &N::TransactionID) -> Result<()> {
        // Ensure the transaction exists.
        if !self.transactions.contains_key(transaction_id)? {
            Err(anyhow!("Transaction {} does not exist in storage", transaction_id))
        } else {
            // Retrieve the transition IDs from the transaction.
            let transition_ids = match self.transactions.get(transaction_id)? {
                Some((_, transition_ids, _)) => transition_ids,
                None => return Err(anyhow!("Transaction {} missing from transactions map", transaction_id)),
            };

            // Remove the transaction entry.
            self.transactions.remove(transaction_id)?;

            for (_, transition_id) in transition_ids.iter().enumerate() {
                // Retrieve the transition from the transition ID.
                let transition = match self.transitions.get(transition_id)? {
                    Some((_, _, transition)) => transition,
                    None => return Err(anyhow!("Transition {} missing from transitions map", transition_id)),
                };

                // Remove the transition.
                self.transitions.remove(transition_id)?;

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
                    self.ciphertext_ids.remove(&*ciphertext_id?)?;
                }
            }

            // Remove the transaction events.
            self.events.remove(transaction_id)?;

            Ok(())
        }
    }
}
