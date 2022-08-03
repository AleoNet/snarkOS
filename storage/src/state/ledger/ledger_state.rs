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
    state::ledger::{genesis_block, Metadata},
    storage::{rocksdb::RocksDB, DataID, DataMap, StorageAccess},
};
use snarkos_environment::helpers::{BlockLocators, Resource, MAXIMUM_LINEAR_BLOCK_LOCATORS, MAXIMUM_QUADRATIC_BLOCK_LOCATORS};
use snarkvm::{
    compiler::{Block, Deployment, Header, Map, MapReader, Transaction, Transactions, Transition},
    console::{account::Signature, program::ProgramID, types::field::Field},
    prelude::Network,
};

use anyhow::{anyhow, Result};
use circular_queue::CircularQueue;
use itertools::Itertools;
use parking_lot::RwLock;
use std::{collections::BTreeMap, marker::PhantomData, path::Path, sync::Arc, thread};
use time::OffsetDateTime;
use tokio::sync::oneshot::{self, error::TryRecvError};

pub(crate) type InternalLedger<N> = snarkvm::prelude::Ledger<
    N,
    DataMap<u32, <N as Network>::BlockHash>,
    DataMap<u32, Header<N>>,
    DataMap<u32, Transactions<N>>,
    DataMap<u32, Signature<N>>,
>;

pub struct LedgerState<N: Network, SA: StorageAccess> {
    /// The latest block of the ledger.
    latest_block: RwLock<Block<N>>,
    /// The latest block hashes and headers in the ledger.
    latest_block_hashes_and_headers: RwLock<CircularQueue<(N::BlockHash, Header<N>)>>,
    /// The block locators from the latest block of the ledger.
    latest_block_locators: RwLock<BlockLocators<N>>,
    /// The state root corresponding to each block height.
    state_roots: RwLock<DataMap<Field<N>, u32>>,
    /// The internal ledger.
    ledger: RwLock<InternalLedger<N>>,
    _storage_access: PhantomData<SA>,
}

impl<N: Network, SA: StorageAccess> LedgerState<N, SA> {
    ///
    /// Opens a read-only instance of `LedgerState` from the given storage path.
    /// For a writable instance of `LedgerState`, use `LedgerState::open_writer`.
    ///
    /// A writable instance of `LedgerState` possesses full functionality, whereas
    /// a read-only instance of `LedgerState` may only call immutable methods.
    ///
    pub fn open_reader<P: AsRef<Path>>(path: P) -> Result<(Arc<Self>, Resource)> {
        // Open storage.
        let context = N::ID;
        let storage = RocksDB::open_read_only(path, context)?;

        // Initialize the ledger.
        let ledger = Arc::new(Self {
            latest_block: RwLock::new(genesis_block::<N>()),
            latest_block_hashes_and_headers: RwLock::new(CircularQueue::<(N::BlockHash, Header<N>)>::with_capacity(
                MAXIMUM_LINEAR_BLOCK_LOCATORS as usize,
            )),
            latest_block_locators: Default::default(),
            state_roots: RwLock::new(storage.open_map(DataID::LedgerRoots)?),
            ledger: RwLock::new(InternalLedger::load(
                storage.open_map(DataID::BlockHashes)?,
                storage.open_map(DataID::BlockHeaders)?,
                storage.open_map(DataID::Transactions)?,
                storage.open_map(DataID::Signatures)?,
            )?),
            _storage_access: PhantomData,
        });

        // Determine the latest block height.
        let latest_block_height = match (ledger.state_roots.read().values().max(), ledger.ledger.read().latest_height()) {
            (Some(latest_block_height_0), latest_block_height_1) => match *latest_block_height_0 == latest_block_height_1 {
                true => *latest_block_height_0,
                false => {
                    return Err(anyhow!(
                        "Ledger storage state is incorrect, use `LedgerState::open_writer` to attempt to automatically fix the problem"
                    ));
                }
            },
            (None, 0) => 0u32,
            _ => return Err(anyhow!("Ledger storage state is inconsistent")),
        };

        // Update the latest ledger state.
        let latest_block = ledger.get_block(latest_block_height)?;
        *ledger.latest_block.write() = latest_block.clone();
        ledger.regenerate_latest_ledger_state()?;

        let resource = ledger.initialize_reader_heartbeat(latest_block)?;

        trace!("[Read-Only] Ledger successfully loaded at block {}", ledger.latest_block_height());
        Ok((ledger, resource))
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Block<N> {
        self.latest_block.read().clone()
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.latest_block.read().header().height()
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.latest_block.read().hash()
    }

    /// Returns the latest block timestamp.
    pub fn latest_block_timestamp(&self) -> i64 {
        self.latest_block.read().header().timestamp()
    }

    /// Returns the latest block coinbase target.
    pub fn latest_coinbase_target(&self) -> u64 {
        self.latest_block.read().header().coinbase_target()
    }

    /// Returns the latest block proof target.
    pub fn latest_proof_target(&self) -> u64 {
        self.latest_block.read().header().proof_target()
    }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> Header<N> {
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

    // /// Returns `true` if the given ledger root exists in storage.
    // pub fn contains_ledger_root(&self, ledger_root: &N::LedgerRoot) -> Result<bool> {
    //     Ok(*ledger_root == self.latest_ledger_root() || self.ledger_roots.contains_key(ledger_root)?)
    // }

    /// Returns `true` if the given block height exists in storage.
    pub fn contains_block_height(&self, block_height: u32) -> Result<bool> {
        self.ledger.read().contains_height(block_height)
    }

    /// Returns `true` if the given block hash exists in storage.
    pub fn contains_block_hash(&self, block_hash: &N::BlockHash) -> Result<bool> {
        Ok(self.ledger.read().contains_block_hash(block_hash))
    }

    /// Returns `true` if the given transaction ID exists in storage.
    pub fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        Ok(self.ledger.read().contains_transaction_id(transaction_id))
    }

    /// Returns `true` if the given serial number exists in storage.
    pub fn contains_serial_number(&self, serial_number: &Field<N>) -> Result<bool> {
        Ok(self.ledger.read().contains_serial_number(serial_number))
    }

    /// Returns `true` if the given commitment exists in storage.
    pub fn contains_commitment(&self, commitment: &Field<N>) -> Result<bool> {
        Ok(self.ledger.read().contains_commitment(commitment))
    }

    /// Returns the transition for a given transition ID.
    pub fn get_transition(&self, transition_id: &Field<N>) -> Result<Transition<N>> {
        // self.ledger.get_transition(transition_id)
        unimplemented!()
    }

    /// Returns the transaction for a given transaction ID.
    pub fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        // self.ledger.get_transaction(transaction_id)
        unimplemented!()
    }

    /// Returns the transaction metadata for a given transaction ID.
    pub fn get_transaction_metadata(&self, _transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        // self.ledger.get_transaction_metadata(transaction_id)
        unimplemented!()
    }

    /// Returns the block height for the given block hash.
    pub fn get_block_height(&self, block_hash: &N::BlockHash) -> Result<u32> {
        // self.ledger.get_block_height(block_hash)
        unimplemented!()
    }

    /// Returns the block hash for the given block height.
    pub fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.ledger.read().get_hash(block_height)
    }

    /// Returns the block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        let mut hashes = Vec::new();
        for height in start_block_height..=end_block_height {
            hashes.push(self.ledger.read().get_hash(height)?);
        }
        Ok(hashes)
    }

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.ledger.read().get_previous_hash(block_height)
    }

    /// Returns the block header for the given block height.
    pub fn get_block_header(&self, block_height: u32) -> Result<Header<N>> {
        Ok(*self.ledger.read().get_header(block_height)?)
    }

    /// Returns the block headers from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_headers(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Header<N>>> {
        let mut headers = Vec::new();
        for height in start_block_height..=end_block_height {
            headers.push(*self.ledger.read().get_header(height)?);
        }
        Ok(headers)
    }

    /// Returns the transactions from the block of the given block height.
    pub fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        Ok(self.ledger.read().get_transactions(block_height)?.into_owned())
    }

    /// Returns the block for a given block height.
    pub fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        self.ledger.read().get_block(block_height)
    }

    /// Returns the blocks from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        let mut blocks = Vec::new();
        for height in start_block_height..=end_block_height {
            blocks.push(self.get_block(height)?);
        }
        Ok(blocks)
    }

    /// Returns the state root in the block header of the given block height.
    pub fn get_previous_state_root(&self, block_height: u32) -> Result<Field<N>> {
        Ok(*(self.ledger.read().get_header(block_height)?).previous_state_root())
    }

    /// Returns the block locators of the current ledger, from the given block height.
    pub fn get_block_locators(&self, block_height: u32) -> Result<BlockLocators<N>> {
        // Initialize the current block height that a block locator is obtained from.
        let mut block_locator_height = block_height;

        // Determine the number of latest block headers to include as block locators (linear).
        let num_block_headers = std::cmp::min(MAXIMUM_LINEAR_BLOCK_LOCATORS, block_locator_height);

        // Construct the list of block locator headers.
        let block_locator_headers = self
            .latest_block_hashes_and_headers
            .read()
            .asc_iter()
            .filter(|(_, header)| header.height() != 0) // Skip the genesis block.
            .take(num_block_headers as usize)
            .cloned()
            .map(|(hash, header)| (header.height(), (hash, Some(header))))
            .collect::<Vec<_>>();

        // Decrement the block locator height by the number of block headers.
        block_locator_height -= num_block_headers;

        // Return the block locators if the locator has run out of blocks.
        if block_locator_height == 0 {
            // Initialize the list of block locators.
            let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<Header<N>>)> = block_locator_headers.into_iter().collect();
            // Add the genesis locator.
            block_locators.insert(0, (self.get_block_hash(0)?, None));

            return BlockLocators::<N>::from(block_locators);
        }

        // Determine the number of latest block hashes to include as block locators (power of two).
        let num_block_hashes = std::cmp::min(MAXIMUM_QUADRATIC_BLOCK_LOCATORS, block_locator_height);

        // Initialize list of block locator hashes.
        let mut block_locator_hashes = Vec::with_capacity(num_block_hashes as usize);
        let mut accumulator = 1;
        // Add the block locator hashes.
        while block_locator_height > 0 && block_locator_hashes.len() < num_block_hashes as usize {
            block_locator_hashes.push((block_locator_height, (self.get_block_hash(block_locator_height)?, None)));

            // Decrement the block locator height by a power of two.
            block_locator_height = block_locator_height.saturating_sub(accumulator);
            accumulator *= 2;
        }

        // Initialize the list of block locators.
        let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<Header<N>>)> =
            block_locator_headers.into_iter().chain(block_locator_hashes).collect();
        // Add the genesis locator.
        block_locators.insert(0, (self.get_block_hash(0)?, None));

        BlockLocators::<N>::from(block_locators)
    }

    /// Check that the block locators are well formed.
    pub fn check_block_locators(&self, block_locators: &BlockLocators<N>) -> Result<bool> {
        // Ensure the genesis block locator exists and is well-formed.
        let (expected_genesis_block_hash, expected_genesis_header) = match block_locators.get(&0) {
            Some((expected_genesis_block_hash, expected_genesis_header)) => (expected_genesis_block_hash, expected_genesis_header),
            None => return Ok(false),
        };
        let genesis_block_hash = self.ledger.read().get_block(0)?.hash();
        if expected_genesis_block_hash != &genesis_block_hash || expected_genesis_header.is_some() {
            return Ok(false);
        }

        let num_linear_block_headers = std::cmp::min(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize, block_locators.len() - 1);
        let num_quadratic_block_headers = block_locators.len().saturating_sub(num_linear_block_headers + 1);

        // Check that the block headers are formed correctly (linear).
        let mut last_block_height = match block_locators.keys().max() {
            Some(height) => *height,
            None => return Ok(false),
        };

        for (block_height, (_block_hash, block_header)) in block_locators.iter().rev().take(num_linear_block_headers) {
            // Check that the block height is decrementing.
            match last_block_height == *block_height {
                true => last_block_height = block_height.saturating_sub(1),
                false => return Ok(false),
            }

            // Check that the block header is present.
            let block_header = match block_header {
                Some(header) => header,
                None => return Ok(false),
            };

            // Check the block height matches in the block header.
            if block_height != &block_header.height() {
                return Ok(false);
            }
        }

        // Check that the remaining block hashes are formed correctly (power of two).
        if block_locators.len() > MAXIMUM_LINEAR_BLOCK_LOCATORS as usize {
            // Iterate through all the quadratic ranged block locators excluding the genesis locator.
            let mut previous_block_height = u32::MAX;
            let mut accumulator = 1;

            for (block_height, (_block_hash, block_header)) in block_locators
                .iter()
                .rev()
                .skip(num_linear_block_headers + 1)
                .take(num_quadratic_block_headers - 1)
            {
                // Check that the block heights decrement by a power of two.
                if previous_block_height != u32::MAX && previous_block_height.saturating_sub(accumulator) != *block_height {
                    return Ok(false);
                }

                // Check that there is no block header.
                if block_header.is_some() {
                    return Ok(false);
                }

                previous_block_height = *block_height;
                accumulator *= 2;
            }
        }

        Ok(true)
    }

    /// Updates the latest block hashes and block headers.
    fn regenerate_latest_ledger_state(&self) -> Result<()> {
        // Compute the start block height and end block height (inclusive).
        let end_block_height = self.latest_block_height();
        let start_block_height = end_block_height.saturating_sub(MAXIMUM_LINEAR_BLOCK_LOCATORS - 1);

        // Retrieve the latest block hashes and block headers.
        let block_hashes = self.get_block_hashes(start_block_height, end_block_height)?;
        let block_headers = self.get_block_headers(start_block_height, end_block_height)?;
        assert_eq!(block_hashes.len(), block_headers.len());

        {
            // Acquire the write lock for the latest block hashes and block headers.
            let mut latest_block_hashes_and_headers = self.latest_block_hashes_and_headers.write();

            // Upon success, clear the latest block hashes and block headers.
            latest_block_hashes_and_headers.clear();

            // Add the latest block hashes and block headers.
            for (block_hash, block_header) in block_hashes.into_iter().zip_eq(block_headers) {
                latest_block_hashes_and_headers.push((block_hash, block_header));
            }
        }

        *self.latest_block_locators.write() = self.get_block_locators(end_block_height)?;

        Ok(())
    }

    /// Initializes a heartbeat to keep the ledger reader in sync, with the given starting block height.
    fn initialize_reader_heartbeat(self: &Arc<Self>, mut current_block: Block<N>) -> Result<Resource> {
        let (abort_sender, mut abort_receiver) = oneshot::channel();

        let ledger = self.clone();
        let thread_handle = thread::spawn(move || {
            loop {
                // Check if the thread shouldn't be aborted.
                match abort_receiver.try_recv() {
                    Ok(_) | Err(TryRecvError::Closed) => return,
                    _ => (),
                };

                // Refresh the ledger storage state.
                if ledger.state_roots.write().refresh() {
                    // After catching up the reader, determine the latest block height.

                    let latest_block_height = match ledger.state_roots.read().values().max() {
                        Some(height) => *height,
                        None => ledger.ledger.read().latest_height(),
                    };

                    let current_block_height = current_block.header().height();
                    let current_block_hash = current_block.hash();
                    trace!(
                        "[Read-Only] Updating ledger state from block {} to {}",
                        current_block_height,
                        latest_block_height
                    );

                    // Update the last seen block.
                    let latest_block = ledger.get_block(latest_block_height);
                    match &latest_block {
                        Ok(ref block) => *ledger.latest_block.write() = block.clone(),
                        Err(error) => warn!("[Read-Only] {}", error),
                    };

                    // Regenerate the latest ledger state.
                    if let Err(error) = ledger.regenerate_latest_ledger_state() {
                        warn!("[Read-Only] {}", error);
                    };

                    // Update the last known block in the reader.
                    if let Ok(block) = latest_block {
                        current_block = block;
                    }
                }
                thread::sleep(std::time::Duration::from_secs(6));
            }
        });

        Ok(Resource::Thread(thread_handle, abort_sender))
    }

    ///
    /// Dump the specified number of blocks to the given location.
    ///
    #[cfg(feature = "test")]
    #[allow(dead_code)]
    pub fn dump_blocks<P: AsRef<Path>>(&self, path: P, count: u32) -> Result<()> {
        let mut file = std::fs::File::create(path)?;
        let mut blocks = Vec::with_capacity(count as usize);

        println!("Commencing block dump");
        for i in 1..=count {
            if i % 10 == 0 || count < 10 {
                println!("Dumping block {}/{}", i, count);
            }
            let block = self.get_block(i)?;
            blocks.push(block);
        }
        println!("Block dump complete");

        bincode::serialize_into(&mut file, &blocks)?;

        Ok(())
    }

    pub fn internal_ledger(&self) -> &RwLock<InternalLedger<N>> {
        &self.ledger
    }

    #[cfg(any(test, feature = "test"))]
    pub fn storage(&self) -> RocksDB {
        self.state_roots.read().storage().clone()
    }
}

impl<N: Network, SA: StorageAccess> LedgerState<N, SA> {
    ///
    /// Opens a new writable instance of `LedgerState` from the given storage path.
    /// For a read-only instance of `LedgerState`, use `LedgerState::open_reader`.
    ///
    /// A writable instance of `LedgerState` possesses full functionality, whereas
    /// a read-only instance of `LedgerState` may only call immutable methods.
    ///
    pub fn open_writer<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_writer_with_increment::<P>(path, 10_000)
    }

    /// This function is hidden, as it's intended to be used directly in tests only.
    /// The `validation_increment` parameter determines the number of blocks to be
    /// handled during the incremental validation process.
    #[doc(hidden)]
    pub fn open_writer_with_increment<P: AsRef<Path>>(path: P, validation_increment: u32) -> Result<Self> {
        // Open storage.
        let context = N::ID;
        let storage = RocksDB::open(path, context)?;

        // Initialize the ledger.
        let ledger = Self {
            // ledger_tree: RwLock::new(LedgerTree::<N>::new()?),
            latest_block: RwLock::new(genesis_block::<N>()),
            latest_block_hashes_and_headers: RwLock::new(CircularQueue::<(N::BlockHash, Header<N>)>::with_capacity(
                MAXIMUM_LINEAR_BLOCK_LOCATORS as usize,
            )),
            latest_block_locators: Default::default(),
            state_roots: RwLock::new(storage.open_map(DataID::LedgerRoots)?),
            ledger: RwLock::new(InternalLedger::load(
                storage.open_map(DataID::BlockHashes)?,
                storage.open_map(DataID::BlockHeaders)?,
                storage.open_map(DataID::Transactions)?,
                storage.open_map(DataID::Signatures)?,
            )?),
            _storage_access: PhantomData,
        };

        // Determine the latest block height.
        let latest_block_height = match (ledger.state_roots.read().values().max(), ledger.ledger.read().latest_height()) {
            (Some(latest_block_height_0), latest_block_height_1) => match *latest_block_height_0 == latest_block_height_1 {
                true => *latest_block_height_0,
                // TODO (raychu86): Try to fix inconsistent state.
                // false => match ledger.try_fixing_inconsistent_state(None) {
                //     Ok(current_block_height) => current_block_height,
                //     Err(error) => return Err(error),
                // },
                false => return Err(anyhow!("Ledger storage state is inconsistent")),
            },
            (None, 0) => 0u32,
            _ => return Err(anyhow!("Ledger storage state is inconsistent")),
        };

        // If this is new storage, initialize it with the genesis block.
        if latest_block_height == 0u32 && !ledger.ledger.read().contains_height(0u32)? {
            let genesis = genesis_block::<N>();

            ledger
                .state_roots
                .write()
                .insert(*genesis.header().previous_state_root(), genesis.header().height())?;
            ledger.ledger.write().add_next_block(&genesis)?;
        }

        // // Check that all canonical block headers exist in storage.
        // let count = ledger.ledger.get_block_header_count()?;
        // assert_eq!(count, latest_block_height.saturating_add(1));

        // Iterate and append each block hash from genesis to tip to validate ledger state.
        let mut start_block_height = 0u32;
        while start_block_height <= latest_block_height {
            // Compute the end block height (inclusive) for this iteration.
            let end_block_height = std::cmp::min(start_block_height.saturating_add(validation_increment), latest_block_height);

            // Log the progress of the validation procedure.
            let progress = (end_block_height as f64 / latest_block_height as f64 * 100f64) as u8;
            debug!("Validating the ledger up to block {} ({}%)", end_block_height, progress);

            // Update the starting block height for the next iteration.
            start_block_height = end_block_height.saturating_add(1);
        }

        // Update the latest ledger state.
        *ledger.latest_block.write() = ledger.get_block(latest_block_height)?;
        ledger.regenerate_latest_ledger_state()?;

        info!("Ledger successfully loaded at block {}", ledger.latest_block_height());
        Ok(ledger)
    }

    /// Adds the given block as the next block in the ledger to storage.
    pub fn add_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.write().add_next_block(block)?;

        self.state_roots
            .write()
            .insert(*block.header().previous_state_root(), block.header().height())?;

        // Update the in-memory objects.
        self.latest_block_hashes_and_headers
            .write()
            .push((block.hash(), block.header().clone()));
        *self.latest_block_locators.write() = self.get_block_locators(block.header().height())?;
        *self.latest_block.write() = block.clone();

        Ok(())
    }

    // /// Attempts to automatically resolve inconsistent ledger state.
    // fn try_fixing_inconsistent_state(&self, batch: Option<usize>) -> Result<u32> {
    //     // Remember whether this operation is within an existing batch.
    //     let is_part_of_a_batch = batch.is_some();
    //
    //     // Determine the latest block height.
    //     match (*self.state_roots.values().max(), *self.ledger.block_heights.keys().max()) {
    //         (Some(latest_block_height_0), Some(latest_block_height_1)) => match latest_block_height_0 == latest_block_height_1 {
    //             true => Ok(latest_block_height_0),
    //             false => {
    //                 // Attempt to resolve the inconsistent state.
    //                 if latest_block_height_0 > latest_block_height_1 {
    //                     debug!("Attempting to automatically resolve inconsistent ledger state");
    //                     // Set the starting block height as the height of the ledger roots block height.
    //                     let mut current_block_height = latest_block_height_0;
    //
    //                     // Perform all the associated storage operations as an atomic batch if it's not part of a batch yet.
    //                     let batch = if let Some(id) = batch {
    //                         id
    //                     } else {
    //                         self.state_roots.prepare_batch()
    //                     };
    //
    //                     // Decrement down to the block height stored in the block heights map.
    //                     while current_block_height > latest_block_height_1 {
    //                         // Find the corresponding ledger root that was not removed.
    //                         let mut candidate_ledger_root = None;
    //                         // Attempt to find the previous ledger root corresponding to the current block height.
    //                         for (previous_ledger_root, block_height) in self.state_roots.iter() {
    //                             // If found, set the previous ledger root, and break.
    //                             if block_height == current_block_height {
    //                                 candidate_ledger_root = Some(previous_ledger_root);
    //                                 break;
    //                             }
    //                         }
    //
    //                         // Update the internal state of the ledger roots, if a candidate was found.
    //                         if let Some(previous_ledger_root) = candidate_ledger_root {
    //                             self.state_roots.remove(&previous_ledger_root, Some(batch))?;
    //                             current_block_height = current_block_height.saturating_sub(1);
    //                         } else {
    //                             // Discard the in-progress batch if it's a standalone operation.
    //                             if !is_part_of_a_batch {
    //                                 self.state_roots.discard_batch(batch)?;
    //                             }
    //
    //                             return Err(anyhow!(
    //                                 "Loaded a ledger with inconsistent state ({} != {}) (failed to automatically resolve)",
    //                                 current_block_height,
    //                                 latest_block_height_1
    //                             ));
    //                         }
    //                     }
    //
    //                     // Execute the pending storage batch if it's a standalone operation.
    //                     if !is_part_of_a_batch {
    //                         self.state_roots.execute_batch(batch)?;
    //                     }
    //
    //                     // If this is reached, the inconsistency was automatically resolved,
    //                     // proceed to return the new block height and continue on.
    //                     debug!("Successfully resolved inconsistent ledger state");
    //                     Ok(current_block_height)
    //                 } else {
    //                     Err(anyhow!(
    //                         "Loaded a ledger with inconsistent state ({} != {}) (unable to automatically resolve)",
    //                         latest_block_height_0,
    //                         latest_block_height_1
    //                     ))
    //                 }
    //             }
    //         },
    //         (None, None) => Ok(0u32),
    //         _ => Err(anyhow!("Ledger storage state is inconsistent")),
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        rocksdb::{tests::temp_dir, RocksDB},
        ReadOnly,
        ReadWrite,
    };
    use snarkvm::prelude::Testnet3;

    type CurrentNetwork = Testnet3;
    type A = snarkvm::circuit::AleoV0;

    #[test]
    fn test_open_ledger_state_reader() {
        let dir = temp_dir();
        {
            let _block_state = LedgerState::<CurrentNetwork, ReadWrite>::open_writer::<_>(&dir).expect("Failed to open ledger state");
        }

        let _block_state = LedgerState::<CurrentNetwork, ReadWrite>::open_reader::<_>(dir).expect("Failed to open ledger state");
    }

    #[test]
    fn test_open_ledger_state_writer() {
        let _block_state = LedgerState::<CurrentNetwork, ReadWrite>::open_writer::<_>(temp_dir()).expect("Failed to open ledger state");
    }
}
