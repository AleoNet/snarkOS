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

#[cfg(any(test, feature = "test"))]
use crate::storage::rocksdb::RocksDB;
use crate::{
    state::ledger::{block_state::BlockState, Metadata},
    storage::{DataID, DataMap, MapRead, MapReadWrite, Storage, StorageAccess, StorageReadWrite},
};
use snarkos_consensus::genesis_block;
use snarkos_environment::helpers::Resource;
use snarkvm::{
    circuit::Aleo,
    compiler::{Block, BlockHeader, Transaction, Transactions, Transition},
    console::types::field::Field,
    prelude::{Address, Network, Record, Visibility},
};
// use snarkos_network::helpers::block_locators::*;

use anyhow::{anyhow, Result};
use circular_queue::CircularQueue;
use itertools::Itertools;
use parking_lot::RwLock;
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    thread,
};
use time::OffsetDateTime;
use tokio::sync::oneshot::{self, error::TryRecvError};

// TODO (raychu86): Fetch MAXIMUM_LINEAR_BLOCK_LOCATORS from config.
const MAXIMUM_LINEAR_BLOCK_LOCATORS: u32 = 64;

#[derive(Debug)]
pub struct LedgerState<N: Network, SA: StorageAccess, A: Aleo<Network = N, BaseField = N::Field>> {
    // /// The current ledger tree of block hashes.
    // ledger_tree: RwLock<LedgerTree<N>>,
    /// The latest block of the ledger.
    latest_block: RwLock<Block<N>>,
    /// The latest block hashes and headers in the ledger.
    latest_block_hashes_and_headers: RwLock<CircularQueue<(N::BlockHash, BlockHeader<N>)>>,
    // /// The block locators from the latest block of the ledger.
    // latest_block_locators: RwLock<BlockLocators<N>>,
    /// The ledger root corresponding to each block height.
    ledger_roots: DataMap<Field<N>, u32, SA>,
    /// The blocks of the ledger in storage.
    blocks: BlockState<N, SA, A>,
}

impl<N: Network, SA: StorageAccess, A: Aleo<Network = N, BaseField = N::Field>> LedgerState<N, SA, A> {
    ///
    /// Opens a read-only instance of `LedgerState` from the given storage path.
    /// For a writable instance of `LedgerState`, use `LedgerState::open_writer`.
    ///
    /// A writable instance of `LedgerState` possesses full functionality, whereas
    /// a read-only instance of `LedgerState` may only call immutable methods.
    ///
    pub fn open_reader<S: Storage<Access = SA>, P: AsRef<Path>>(path: P) -> Result<(Arc<Self>, Resource)> {
        // Open storage.
        let context = N::ID;
        let storage = S::open(path, context)?;

        // Initialize the ledger.
        let ledger = Arc::new(Self {
            // ledger_tree: RwLock::new(LedgerTree::<N>::new()?),
            latest_block: RwLock::new(genesis_block::<N, A>()?.clone()),
            latest_block_hashes_and_headers: RwLock::new(CircularQueue::with_capacity(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize)),
            // latest_block_locators: Default::default(),
            ledger_roots: storage.open_map(DataID::LedgerRoots)?,
            blocks: BlockState::<_, _, A>::open(storage)?,
        });

        // Determine the latest block height.
        let latest_block_height = match (ledger.ledger_roots.values().max(), ledger.blocks.block_heights.keys().max()) {
            (Some(latest_block_height_0), Some(latest_block_height_1)) => match latest_block_height_0 == latest_block_height_1 {
                true => latest_block_height_0,
                false => {
                    return Err(anyhow!(
                        "Ledger storage state is incorrect, use `LedgerState::open_writer` to attempt to automatically fix the problem"
                    ));
                }
            },
            (None, None) => 0u32,
            _ => return Err(anyhow!("Ledger storage state is inconsistent")),
        };

        // Update the latest ledger state.
        let latest_block = ledger.get_block(latest_block_height)?;
        *ledger.latest_block.write() = latest_block.clone();
        ledger.regenerate_latest_ledger_state()?;

        // TODO (raychu86): Reintroduce ledger tree
        // // Update the ledger tree state.
        // ledger.regenerate_ledger_tree()?;
        // As the ledger is in read-only mode, proceed to start a process to keep the reader in sync.
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

    // /// Returns the latest cumulative weight.
    // pub fn latest_cumulative_weight(&self) -> u128 {
    //     self.latest_block.read().cumulative_weight()
    // }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> BlockHeader<N> {
        self.latest_block.read().header().clone()
    }

    /// Returns the transactions from the latest block.
    pub fn latest_block_transactions(&self) -> Transactions<N> {
        self.latest_block.read().transactions().clone()
    }

    // /// Returns the latest block locators.
    // pub fn latest_block_locators(&self) -> BlockLocators<N> {
    //     self.latest_block_locators.read().clone()
    // }

    // /// Returns the latest ledger root.
    // pub fn latest_ledger_root(&self) -> Field<N> {
    //     self.ledger_tree.read().root()
    // }

    // /// Returns `true` if the given ledger root exists in storage.
    // pub fn contains_ledger_root(&self, ledger_root: &N::LedgerRoot) -> Result<bool> {
    //     Ok(*ledger_root == self.latest_ledger_root() || self.ledger_roots.contains_key(ledger_root)?)
    // }

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
    pub fn contains_serial_number(&self, serial_number: &Field<N>) -> Result<bool> {
        self.blocks.contains_serial_number(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub fn contains_commitment(&self, commitment: &Field<N>) -> Result<bool> {
        self.blocks.contains_commitment(commitment)
    }

    // /// Returns the record ciphertext for a given commitment.
    // pub fn get_ciphertext(&self, commitment: &N::Commitment) -> Result<N::RecordCiphertext> {
    //     self.blocks.get_ciphertext(commitment)
    // }

    /// Returns the transition for a given transition ID.
    pub fn get_transition(&self, transition_id: &Field<N>) -> Result<Transition<N>> {
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

    // /// Returns the cumulative weight up to a given block height (inclusive) for the canonical chain.
    // pub fn get_cumulative_weight(&self, block_height: u32) -> Result<u128> {
    //     self.blocks.get_cumulative_weight(block_height)
    // }

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
    pub fn get_previous_ledger_root(&self, block_height: u32) -> Result<Field<N>> {
        self.blocks.get_previous_ledger_root(block_height)
    }

    // /// Returns the block locators of the current ledger, from the given block height.
    // pub fn get_block_locators(&self, block_height: u32) -> Result<BlockLocators<N>> {
    //     // Initialize the current block height that a block locator is obtained from.
    //     let mut block_locator_height = block_height;
    //
    //     // Determine the number of latest block headers to include as block locators (linear).
    //     let num_block_headers = std::cmp::min(MAXIMUM_LINEAR_BLOCK_LOCATORS, block_locator_height);
    //
    //     // Construct the list of block locator headers.
    //     let block_locator_headers = self
    //         .latest_block_hashes_and_headers
    //         .read()
    //         .asc_iter()
    //         .filter(|(_, header)| header.height() != 0) // Skip the genesis block.
    //         .take(num_block_headers as usize)
    //         .cloned()
    //         .map(|(hash, header)| (header.height(), (hash, Some(header))))
    //         .collect::<Vec<_>>();
    //
    //     // Decrement the block locator height by the number of block headers.
    //     block_locator_height -= num_block_headers;
    //
    //     // Return the block locators if the locator has run out of blocks.
    //     if block_locator_height == 0 {
    //         // Initialize the list of block locators.
    //         let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<BlockHeader<N>>)> = block_locator_headers.into_iter().collect();
    //         // Add the genesis locator.
    //         block_locators.insert(0, (self.get_block_hash(0)?, None));
    //
    //         return BlockLocators::<N>::from(block_locators);
    //     }
    //
    //     // Determine the number of latest block hashes to include as block locators (power of two).
    //     let num_block_hashes = std::cmp::min(MAXIMUM_QUADRATIC_BLOCK_LOCATORS, block_locator_height);
    //
    //     // Initialize list of block locator hashes.
    //     let mut block_locator_hashes = Vec::with_capacity(num_block_hashes as usize);
    //     let mut accumulator = 1;
    //     // Add the block locator hashes.
    //     while block_locator_height > 0 && block_locator_hashes.len() < num_block_hashes as usize {
    //         block_locator_hashes.push((block_locator_height, (self.get_block_hash(block_locator_height)?, None)));
    //
    //         // Decrement the block locator height by a power of two.
    //         block_locator_height = block_locator_height.saturating_sub(accumulator);
    //         accumulator *= 2;
    //     }
    //
    //     // Initialize the list of block locators.
    //     let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<BlockHeader<N>>)> =
    //         block_locator_headers.into_iter().chain(block_locator_hashes).collect();
    //     // Add the genesis locator.
    //     block_locators.insert(0, (self.get_block_hash(0)?, None));
    //
    //     BlockLocators::<N>::from(block_locators)
    // }
    //
    // /// Check that the block locators are well formed.
    // pub fn check_block_locators(&self, block_locators: &BlockLocators<N>) -> Result<bool> {
    //     // Ensure the genesis block locator exists and is well-formed.
    //     let (expected_genesis_block_hash, expected_genesis_header) = match block_locators.get(&0) {
    //         Some((expected_genesis_block_hash, expected_genesis_header)) => (expected_genesis_block_hash, expected_genesis_header),
    //         None => return Ok(false),
    //     };
    //     if expected_genesis_block_hash != &N::genesis_block().hash() || expected_genesis_header.is_some() {
    //         return Ok(false);
    //     }
    //
    //     let num_linear_block_headers = std::cmp::min(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize, block_locators.len() - 1);
    //     let num_quadratic_block_headers = block_locators.len().saturating_sub(num_linear_block_headers + 1);
    //
    //     // Check that the block headers are formed correctly (linear).
    //     let mut last_block_height = match block_locators.keys().max() {
    //         Some(height) => *height,
    //         None => return Ok(false),
    //     };
    //
    //     for (block_height, (_block_hash, block_header)) in block_locators.iter().rev().take(num_linear_block_headers) {
    //         // Check that the block height is decrementing.
    //         match last_block_height == *block_height {
    //             true => last_block_height = block_height.saturating_sub(1),
    //             false => return Ok(false),
    //         }
    //
    //         // Check that the block header is present.
    //         let block_header = match block_header {
    //             Some(header) => header,
    //             None => return Ok(false),
    //         };
    //
    //         // Check the block height matches in the block header.
    //         if block_height != &block_header.height() {
    //             return Ok(false);
    //         }
    //     }
    //
    //     // Check that the remaining block hashes are formed correctly (power of two).
    //     if block_locators.len() > MAXIMUM_LINEAR_BLOCK_LOCATORS as usize {
    //         // Iterate through all the quadratic ranged block locators excluding the genesis locator.
    //         let mut previous_block_height = u32::MAX;
    //         let mut accumulator = 1;
    //
    //         for (block_height, (_block_hash, block_header)) in block_locators
    //             .iter()
    //             .rev()
    //             .skip(num_linear_block_headers + 1)
    //             .take(num_quadratic_block_headers - 1)
    //         {
    //             // Check that the block heights decrement by a power of two.
    //             if previous_block_height != u32::MAX && previous_block_height.saturating_sub(accumulator) != *block_height {
    //                 return Ok(false);
    //             }
    //
    //             // Check that there is no block header.
    //             if block_header.is_some() {
    //                 return Ok(false);
    //             }
    //
    //             previous_block_height = *block_height;
    //             accumulator *= 2;
    //         }
    //     }
    //
    //     Ok(true)
    // }
    //
    // /// Returns a block template based on the latest state of the ledger.
    // pub fn get_block_template<R: Rng + CryptoRng>(
    //     &self,
    //     recipient: Address<N>,
    //     is_public: bool,
    //     transactions: &[Transaction<N>],
    //     rng: &mut R,
    // ) -> Result<BlockTemplate<N>> {
    //     // Fetch the latest state of the ledger.
    //     let latest_block = self.latest_block();
    //     let previous_ledger_root = self.latest_ledger_root();
    //
    //     // Prepare the new block.
    //     let previous_block_hash = latest_block.hash();
    //     let block_height = latest_block.height().saturating_add(1);
    //     // Ensure that the new timestamp is ahead of the previous timestamp.
    //     let block_timestamp = std::cmp::max(
    //         OffsetDateTime::now_utc().unix_timestamp(),
    //         latest_block.timestamp().saturating_add(1),
    //     );
    //
    //     // Compute the block difficulty target.
    //     let difficulty_target = if N::ID == 3 && block_height <= snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT {
    //         Blocks::<N>::compute_difficulty_target(latest_block.header(), block_timestamp, block_height)
    //     } else if N::ID == 3 {
    //         let anchor_block_header = self.get_block_header(snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT)?;
    //         Blocks::<N>::compute_difficulty_target(&anchor_block_header, block_timestamp, block_height)
    //     } else {
    //         Blocks::<N>::compute_difficulty_target(N::genesis_block().header(), block_timestamp, block_height)
    //     };
    //
    //     // Compute the cumulative weight.
    //     let cumulative_weight = latest_block
    //         .cumulative_weight()
    //         .saturating_add((u64::MAX / difficulty_target) as u128);
    //
    //     // Compute the coinbase reward (not including the transaction fees).
    //     let mut coinbase_reward = Block::<N>::block_reward(block_height);
    //     let mut transaction_fees = AleoAmount::ZERO;
    //
    //     // Filter the transactions to ensure they are new, and append the coinbase transaction.
    //     let mut transactions: Vec<Transaction<N>> = transactions
    //         .iter()
    //         .filter(|transaction| {
    //             for serial_number in transaction.serial_numbers() {
    //                 if let Ok(true) = self.contains_serial_number(serial_number) {
    //                     trace!(
    //                         "Ledger is filtering out transaction {} (serial_number {})",
    //                         transaction.transaction_id(),
    //                         serial_number
    //                     );
    //                     return false;
    //                 }
    //             }
    //             for commitment in transaction.commitments() {
    //                 if let Ok(true) = self.contains_commitment(commitment) {
    //                     trace!(
    //                         "Ledger is filtering out transaction {} (commitment {})",
    //                         transaction.transaction_id(),
    //                         commitment
    //                     );
    //                     return false;
    //                 }
    //             }
    //             trace!("Adding transaction {} to block template", transaction.transaction_id());
    //             transaction_fees = transaction_fees.add(transaction.value_balance());
    //             true
    //         })
    //         .cloned()
    //         .collect();
    //
    //     // Enforce that the transaction fee is positive or zero.
    //     if transaction_fees.is_negative() {
    //         return Err(anyhow!("Invalid transaction fees"));
    //     }
    //
    //     // Calculate the final coinbase reward (including the transaction fees).
    //     coinbase_reward = coinbase_reward.add(transaction_fees);
    //
    //     // Craft a coinbase transaction, and append it to the list of transactions.
    //     let (coinbase_transaction, coinbase_record) = Transaction::<N>::new_coinbase(recipient, coinbase_reward, is_public, rng)?;
    //     transactions.push(coinbase_transaction);
    //
    //     // Construct the new block transactions.
    //     let transactions = Transactions::from(&transactions)?;
    //
    //     // Construct the block template.
    //     Ok(BlockTemplate::new(
    //         previous_block_hash,
    //         block_height,
    //         block_timestamp,
    //         difficulty_target,
    //         cumulative_weight,
    //         previous_ledger_root,
    //         transactions,
    //         coinbase_record,
    //     ))
    // }
    //
    // ///
    // /// Returns a ledger proof for the given commitment.
    // ///
    // pub fn get_ledger_inclusion_proof(&self, commitment: N::Commitment) -> Result<LedgerProof<N>> {
    //     // TODO (raychu86): Add getter functions.
    //     let commitment_transition_id = match self.blocks.transactions.commitments.get(&commitment)? {
    //         Some(transition_id) => transition_id,
    //         None => return Err(anyhow!("commitment {} missing from commitments map", commitment)),
    //     };
    //
    //     let transaction_id = match self.blocks.transactions.transitions.get(&commitment_transition_id)? {
    //         Some((transaction_id, _, _)) => transaction_id,
    //         None => return Err(anyhow!("transition id {} missing from transactions map", commitment_transition_id)),
    //     };
    //
    //     let transaction = self.get_transaction(&transaction_id)?;
    //
    //     let block_hash = match self.blocks.transactions.transactions.get(&transaction_id)? {
    //         Some((_, _, metadata)) => metadata.block_hash,
    //         None => return Err(anyhow!("transaction id {} missing from transactions map", transaction_id)),
    //     };
    //
    //     let block_header = match self.blocks.block_headers.get(&block_hash)? {
    //         Some(block_header) => block_header,
    //         None => return Err(anyhow!("Block {} missing from block headers map", block_hash)),
    //     };
    //
    //     // Generate the local proof for the commitment.
    //     let local_proof = transaction.to_local_proof(commitment)?;
    //
    //     let transaction_id = local_proof.transaction_id();
    //     let transactions = self.get_block_transactions(block_header.height())?;
    //
    //     // Compute the transactions inclusion proof.
    //     let transactions_inclusion_proof = {
    //         let index = transactions.transaction_ids().position(|id| id == transaction_id).unwrap();
    //         transactions.to_transactions_inclusion_proof(index, transaction_id)?
    //     };
    //
    //     // Compute the block header inclusion proof.
    //     let transactions_root = transactions.transactions_root();
    //     let block_header_inclusion_proof = block_header.to_header_inclusion_proof(1, transactions_root)?;
    //     let block_header_root = block_header.to_header_root()?;
    //
    //     // Determine the previous block hash.
    //     let previous_block_hash = self.get_previous_block_hash(self.get_block_height(&block_hash)?)?;
    //
    //     // Generate the record proof.
    //     let record_proof = RecordProof::new(
    //         block_hash,
    //         previous_block_hash,
    //         block_header_root,
    //         block_header_inclusion_proof,
    //         transactions_root,
    //         transactions_inclusion_proof,
    //         local_proof,
    //     )?;
    //
    //     // Generate the ledger root inclusion proof.
    //     let ledger_root = self.ledger_tree.read().root();
    //     let ledger_root_inclusion_proof = self.ledger_tree.read().to_ledger_inclusion_proof(&block_hash)?;
    //
    //     LedgerProof::new(ledger_root, ledger_root_inclusion_proof, record_proof)
    // }

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

        // TODO (raychu86): Reintroduce block locators
        // *self.latest_block_locators.write() = self.get_block_locators(end_block_height)?;

        Ok(())
    }

    // /// Regenerates the ledger tree.
    // fn regenerate_ledger_tree(&self) -> Result<()> {
    //     // Acquire the ledger tree write lock.
    //     let mut ledger_tree = self.ledger_tree.write();
    //
    //     // Retrieve all of the block hashes.
    //     let block_hashes = self.get_block_hashes(0, self.latest_block_height())?;
    //
    //     // Add the block hashes to create the new ledger tree.
    //     let mut new_ledger_tree = LedgerTree::<N>::new()?;
    //     new_ledger_tree.add_all(&block_hashes)?;
    //
    //     // Update the current ledger tree with the current state.
    //     *ledger_tree = new_ledger_tree;
    //
    //     Ok(())
    // }
    //
    // /// Updates the ledger tree.
    // fn update_ledger_tree(&self, outdated_block_height: u32, new_block_height: u32) -> Result<()> {
    //     // Acquire the ledger tree write lock.
    //     let mut ledger_tree = self.ledger_tree.write();
    //
    //     let mut new_ledger_tree = ledger_tree.clone();
    //
    //     // Retrieve all the new block hashes.
    //     let block_hashes = self.get_block_hashes(outdated_block_height + 1, new_block_height)?;
    //
    //     // Add the block hashes to create the new ledger tree.
    //     new_ledger_tree.add_all(&block_hashes)?;
    //
    //     // Update the current ledger tree with the current state.
    //     *ledger_tree = new_ledger_tree;
    //
    //     Ok(())
    // }

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
                if ledger.ledger_roots.refresh() {
                    // After catching up the reader, determine the latest block height.
                    if let Some(latest_block_height) = ledger.blocks.block_heights.keys().max() {
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

                        // TODO (raychu86): Reintroduce ledger tree.
                        // // A flag indicating whether a fast ledger tree update is feasible.
                        // let mut quick_update = false;
                        //
                        // // Only consider an update if the latest height is actually greater than the current height.
                        // if latest_block_height > current_block_height {
                        //     // If the last known top block hash still exists at the expected height, there was no rollback
                        //     // beyond it, which means we only need to update the ledger tree with the new hashes.
                        //     if let Ok(found_block_hash) = ledger.get_block_hash(current_block_height) {
                        //         if found_block_hash == current_block_hash {
                        //             // Update the ledger tree.
                        //             if let Err(error) = ledger.update_ledger_tree(current_block_height, latest_block_height) {
                        //                 warn!("[Read-Only] {}", error);
                        //             } else {
                        //                 quick_update = true;
                        //             }
                        //         }
                        //     }
                        // }
                        //
                        // // If a quick ledger tree update was infeasible, regenerate it in its entirety.
                        // if !quick_update {
                        //     // Regenerate the entire ledger tree.
                        //     if let Err(error) = ledger.regenerate_ledger_tree() {
                        //         warn!("[Read-Only] {}", error);
                        //     };
                        // }

                        // Regenerate the latest ledger state.
                        if let Err(error) = ledger.regenerate_latest_ledger_state() {
                            warn!("[Read-Only] {}", error);
                        };

                        // Update the last known block in the reader.
                        if let Ok(block) = latest_block {
                            current_block = block;
                        }
                    }
                }
                thread::sleep(std::time::Duration::from_secs(6));
            }
        });

        Ok(Resource::Thread(thread_handle, abort_sender))
    }

    // /// Proposes a new block to the ledger.
    // pub fn propose_new_block<R: Rng + CryptoRng, Private: Visibility>(
    //     &self,
    //     recipient: Address<N>,
    //     is_public: bool,
    //     transactions: &[Transaction<N>],
    //     rng: &mut R,
    // ) -> Result<(Block<N>, Record<N, Private>)> {
    //     let latest_block_header = self.latest_block().header();
    //
    //     // let template = self.get_block_template(recipient, is_public, transactions, rng)?;
    //     // let coinbase_record = template.coinbase_record().clone();
    //
    //     // Mine the next block.
    //     match Block::mine(&template, terminator, rng) {
    //         Ok(block) => Ok((block, coinbase_record)),
    //         Err(error) => Err(anyhow!("Unable to mine the next block: {}", error)),
    //     }
    // }

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

    #[cfg(any(test, feature = "test"))]
    pub fn storage(&self) -> &RocksDB<SA> {
        self.ledger_roots.storage()
    }
}

impl<N: Network, SA: StorageReadWrite, A: Aleo<Network = N, BaseField = N::Field>> LedgerState<N, SA, A> {
    ///
    /// Opens a new writable instance of `LedgerState` from the given storage path.
    /// For a read-only instance of `LedgerState`, use `LedgerState::open_reader`.
    ///
    /// A writable instance of `LedgerState` possesses full functionality, whereas
    /// a read-only instance of `LedgerState` may only call immutable methods.
    ///
    pub fn open_writer<S: Storage<Access = SA>, P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_writer_with_increment::<S, P>(path, 10_000)
    }

    /// This function is hidden, as it's intended to be used directly in tests only.
    /// The `validation_increment` parameter determines the number of blocks to be
    /// handled during the incremental validation process.
    #[doc(hidden)]
    pub fn open_writer_with_increment<S: Storage<Access = SA>, P: AsRef<Path>>(path: P, validation_increment: u32) -> Result<Self> {
        // Open storage.
        let context = N::ID;
        let storage = S::open(path, context)?;

        // Initialize the ledger.
        let ledger = Self {
            // ledger_tree: RwLock::new(LedgerTree::<N>::new()?),
            latest_block: RwLock::new(genesis_block::<N, A>()?.clone()),
            latest_block_hashes_and_headers: RwLock::new(CircularQueue::with_capacity(MAXIMUM_LINEAR_BLOCK_LOCATORS as usize)),
            // latest_block_locators: Default::default(),
            ledger_roots: storage.open_map(DataID::LedgerRoots)?,
            blocks: BlockState::<_, _, A>::open(storage)?,
        };

        // Determine the latest block height.
        let mut latest_block_height = match (ledger.ledger_roots.values().max(), ledger.blocks.block_heights.keys().max()) {
            (Some(latest_block_height_0), Some(latest_block_height_1)) => match latest_block_height_0 == latest_block_height_1 {
                true => latest_block_height_0,
                false => match ledger.try_fixing_inconsistent_state(None) {
                    Ok(current_block_height) => current_block_height,
                    Err(error) => return Err(error),
                },
            },
            (None, None) => 0u32,
            _ => return Err(anyhow!("Ledger storage state is inconsistent")),
        };

        // If this is new storage, initialize it with the genesis block.
        if latest_block_height == 0u32 && !ledger.blocks.contains_block_height(0u32)? {
            let genesis = genesis_block::<N, A>()?;

            // Perform all the associated storage operations as an atomic batch.
            let batch = ledger.ledger_roots.prepare_batch();

            ledger
                .ledger_roots
                .insert(&genesis.header().previous_ledger_root(), &genesis.header().height(), Some(batch))?;
            ledger.blocks.add_block(&genesis, Some(batch))?;

            // Execute the pending storage batch.
            ledger.ledger_roots.execute_batch(batch)?;
        }

        // Check that all canonical block headers exist in storage.
        let count = ledger.blocks.get_block_header_count()?;
        assert_eq!(count, latest_block_height.saturating_add(1));

        // Iterate and append each block hash from genesis to tip to validate ledger state.
        let mut start_block_height = 0u32;
        while start_block_height <= latest_block_height {
            // Compute the end block height (inclusive) for this iteration.
            let end_block_height = std::cmp::min(start_block_height.saturating_add(validation_increment), latest_block_height);

            // Retrieve the block hashes.
            let block_hashes = ledger.get_block_hashes(start_block_height, end_block_height)?;

            // TODO (raychu86): Reintroduce ledger tree.
            // // Split the block hashes into (last_block_hash, [start_block_hash, ..., penultimate_block_hash]).
            // if let Some((last_block_hash, block_hashes_excluding_last)) = block_hashes.split_last() {
            //     // It's possible that the batch only contains one block.
            //     if !block_hashes_excluding_last.is_empty() {
            //         // Add the block hashes (up to penultimate) to the ledger tree.
            //         ledger.ledger_tree.write().add_all(block_hashes_excluding_last)?;
            //     }
            //
            //     // Check 1 - Ensure the root of the ledger tree matches the one saved in the ledger roots map.
            //     let ledger_root = ledger.get_previous_ledger_root(end_block_height)?;
            //     if ledger_root != ledger.ledger_tree.read().root() {
            //         return Err(anyhow!("Ledger has incorrect ledger tree state at block {}", end_block_height));
            //     }
            //
            //     // Check 2 - Ensure the saved block height corresponding to this ledger root matches the expected block height.
            //     let candidate_height = match ledger.ledger_roots.get(&ledger_root)? {
            //         Some(candidate_height) => candidate_height,
            //         None => return Err(anyhow!("Ledger is missing ledger root for block {}", end_block_height)),
            //     };
            //     if end_block_height != candidate_height {
            //         return Err(anyhow!(
            //             "Ledger expected block {}, found block {}",
            //             end_block_height,
            //             candidate_height
            //         ));
            //     }
            //
            //     // Add the last block hash to the ledger tree.
            //     ledger.ledger_tree.write().add(last_block_hash)?;
            // }

            // Log the progress of the validation procedure.
            let progress = (end_block_height as f64 / latest_block_height as f64 * 100f64) as u8;
            debug!("Validating the ledger up to block {} ({}%)", end_block_height, progress);

            // Update the starting block height for the next iteration.
            start_block_height = end_block_height.saturating_add(1);
        }

        // // If this is new storage, the while loop above did not execute,
        // // and proceed to add the genesis block hash into the ledger tree.
        // if start_block_height == 0u32 {
        //     // Add the genesis block hash to the ledger tree.
        //     ledger.ledger_tree.write().add(&genesis_block::<N, A>()?.hash())?;
        // }

        // Update the latest ledger state.
        *ledger.latest_block.write() = ledger.get_block(latest_block_height)?;
        ledger.regenerate_latest_ledger_state()?;

        // TODO (raychu86): Reintroduce ledger tree
        // Validate the ledger root one final time.
        // let latest_ledger_root = ledger.ledger_tree.read().root();
        // ledger.regenerate_ledger_tree()?;
        // assert_eq!(ledger.ledger_tree.read().root(), latest_ledger_root);

        info!("Ledger successfully loaded at block {}", ledger.latest_block_height());
        Ok(ledger)
    }

    // /// Adds the given block as the next block in the ledger to storage.
    // pub fn add_next_block(&self, block: &Block<N>) -> Result<()> {
    //     // Ensure the block itself is valid.
    //     if !block.is_valid() {
    //         return Err(anyhow!("Block {} is invalid", block.header().height()));
    //     }
    //
    //     // Retrieve the current block.
    //     let current_block = self.latest_block();
    //
    //     // Ensure the block height increments by one.
    //     let block_height = block.header().height();
    //     if block_height != current_block.header().height() + 1 {
    //         return Err(anyhow!(
    //             "Block {} should have block height {}",
    //             block_height,
    //             current_block.height() + 1
    //         ));
    //     }
    //
    //     // Ensure the previous block hash matches.
    //     if block.previous_block_hash() != current_block.hash() {
    //         return Err(anyhow!(
    //             "Block {} has an incorrect previous block hash in the canon chain",
    //             block_height
    //         ));
    //     }
    //
    //     // Ensure the next block timestamp is within the declared time limit.
    //     let now = OffsetDateTime::now_utc().unix_timestamp();
    //     if block.header().timestamp() > (now + N::ALEO_FUTURE_TIME_LIMIT_IN_SECS) {
    //         return Err(anyhow!("The given block timestamp exceeds the time limit"));
    //     }
    //
    //     // Ensure the next block timestamp is after the current block timestamp.
    //     if block.header().timestamp() <= current_block.header().timestamp() {
    //         return Err(anyhow!("The given block timestamp is before the current timestamp"));
    //     }
    //
    //     // Compute the expected difficulty target.
    //     let expected_difficulty_target = if N::ID == 3 && block_height <= snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT {
    //         Blocks::<N>::compute_difficulty_target(current_block.header(), block.header().timestamp(), block.header().height())
    //     } else if N::ID == 3 {
    //         let anchor_block_header = self.get_block_header(snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT)?;
    //         Blocks::<N>::compute_difficulty_target(&anchor_block_header, block.header().timestamp(), block.header().height())
    //     } else {
    //         Blocks::<N>::compute_difficulty_target(N::genesis_block().header(), block.header().timestamp(), block.header().height())
    //     };
    //
    //     // Ensure the expected difficulty target is met.
    //     if block.difficulty_target() != expected_difficulty_target {
    //         return Err(anyhow!(
    //             "Block {} has an incorrect difficulty target. Found {}, but expected {}",
    //             block_height,
    //             block.difficulty_target(),
    //             expected_difficulty_target
    //         ));
    //     }
    //
    //     // Ensure the block height does not already exist.
    //     if self.contains_block_height(block_height)? {
    //         return Err(anyhow!("Block {} already exists in the canon chain", block_height));
    //     }
    //
    //     // Ensure the block hash does not already exist.
    //     if self.contains_block_hash(&block.hash())? {
    //         return Err(anyhow!("Block {} has a repeat block hash in the canon chain", block_height));
    //     }
    //
    //     // Ensure the ledger root in the block matches the current ledger root.
    //     if block.previous_ledger_root() != self.latest_ledger_root() {
    //         return Err(anyhow!("Block {} declares an incorrect ledger root", block_height));
    //     }
    //
    //     // Ensure the canon chain does not already contain the given serial numbers.
    //     for serial_number in block.transactions().serial_numbers() {
    //         if self.contains_serial_number(serial_number)? {
    //             return Err(anyhow!("Serial number {} already exists in the ledger", serial_number));
    //         }
    //     }
    //
    //     // Ensure the canon chain does not already contain the given commitments.
    //     for commitment in block.transactions().commitments() {
    //         if self.contains_commitment(commitment)? {
    //             return Err(anyhow!("Commitment {} already exists in the ledger", commitment));
    //         }
    //     }
    //
    //     // Ensure each transaction in the given block is new to the canon chain.
    //     for transaction in block.transactions().iter() {
    //         // Ensure the transactions in the given block do not already exist.
    //         if self.contains_transaction(&transaction.id())? {
    //             return Err(anyhow!(
    //                 "Transaction {} in block {} has a duplicate transaction in the ledger",
    //                 transaction.id(),
    //                 block_height
    //             ));
    //         }
    //
    //         // TODO (raychu86): Reintroduce ledger root.
    //         // // Ensure the transaction in the block references a valid past or current ledger root.
    //         // if !self.contains_ledger_root(&transaction.ledger_root())? {
    //         //     return Err(anyhow!(
    //         //         "Transaction {} in block {} references non-existent ledger root {}",
    //         //         transaction.id(),
    //         //         block_height,
    //         //         &transaction.ledger_root()
    //         //     ));
    //         // }
    //     }
    //
    //     // Perform all the associated storage operations as an atomic batch.
    //     let batch = self.ledger_roots.prepare_batch();
    //
    //     self.blocks.add_block(block, Some(batch))?;
    //     self.ledger_roots
    //         .insert(&block.header().previous_ledger_root(), &block.header().height(), Some(batch))?;
    //
    //     // Execute the pending storage batch.
    //     self.ledger_roots.execute_batch(batch)?;
    //
    //     // Update the in-memory objects.
    //     // TODO (raychu86): Reintroduce ledger tree.
    //     // self.ledger_tree.write().add(&block.hash())?;
    //     self.latest_block_hashes_and_headers
    //         .write()
    //         .push((block.hash(), block.header().clone()));
    //     // TODO (raychu86): Reintroduce block locators.
    //     // *self.latest_block_locators.write() = self.get_block_locators(block.height())?;
    //     *self.latest_block.write() = block.clone();
    //
    //     Ok(())
    // }

    /// Reverts the ledger state back to the given block height, returning the removed blocks on success.
    pub fn revert_to_block_height(&self, block_height: u32) -> Result<Vec<Block<N>>> {
        // Determine the number of blocks to remove.
        let latest_block_height = self.latest_block_height();
        let number_of_blocks = latest_block_height.saturating_sub(block_height);

        // TODO (raychu86): Fetch ALEO_MAXIMUM_FORK_DEPTH from config.
        const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;
        // Ensure the reverted block height is within a permitted range and well-formed.
        if block_height >= latest_block_height || number_of_blocks > ALEO_MAXIMUM_FORK_DEPTH || self.get_block(block_height).is_err() {
            return Err(anyhow!("Attempted to return to block height {}, which is invalid", block_height));
        }

        // Fetch the blocks to be removed. This ensures the blocks to be removed exist in the ledger,
        // and is used during the removal process to expedite the procedure.
        let start_block_height = latest_block_height.saturating_sub(number_of_blocks);
        let blocks: BTreeMap<u32, Block<N>> = self
            .get_blocks(start_block_height, latest_block_height)?
            .iter()
            .map(|block| (block.header().height(), block.clone()))
            .collect();

        // Perform all the associated storage operations as an atomic batch.
        let batch = self.ledger_roots.prepare_batch();

        // Process the block removals.
        let mut current_block_height = latest_block_height;
        let mut current_block = blocks.get(&current_block_height);
        while current_block_height > block_height {
            match current_block {
                Some(block) => {
                    // Update the internal storage state of the ledger.
                    self.blocks.remove_block(current_block_height, Some(batch))?;
                    self.ledger_roots.remove(&block.header().previous_ledger_root(), Some(batch))?;
                    // Decrement the current block height, and update the current block.
                    current_block_height = current_block_height.saturating_sub(1);
                    current_block = blocks.get(&current_block_height);
                }
                None => match self.try_fixing_inconsistent_state(Some(batch)) {
                    Ok(block_height) => {
                        current_block_height = block_height;
                        break;
                    }
                    Err(error) => {
                        self.ledger_roots.discard_batch(batch)?;
                        return Err(error);
                    }
                },
            }
        }

        // Execute the pending storage batch.
        self.ledger_roots.execute_batch(batch)?;

        // Update the latest block.
        *self.latest_block.write() = self.get_block(current_block_height)?;
        // Regenerate the latest ledger state.
        self.regenerate_latest_ledger_state()?;

        // TODO (raychu86): Reintroduce ledger tree.
        // // Regenerate the ledger tree.
        // self.regenerate_ledger_tree()?;

        // Return the removed blocks, in increasing order (i.e. 1, 2, 3...).
        Ok(blocks.values().skip(1).cloned().collect())
    }

    /// Attempts to automatically resolve inconsistent ledger state.
    fn try_fixing_inconsistent_state(&self, batch: Option<usize>) -> Result<u32> {
        // Remember whether this operation is within an existing batch.
        let is_part_of_a_batch = batch.is_some();

        // Determine the latest block height.
        match (self.ledger_roots.values().max(), self.blocks.block_heights.keys().max()) {
            (Some(latest_block_height_0), Some(latest_block_height_1)) => match latest_block_height_0 == latest_block_height_1 {
                true => Ok(latest_block_height_0),
                false => {
                    // Attempt to resolve the inconsistent state.
                    if latest_block_height_0 > latest_block_height_1 {
                        debug!("Attempting to automatically resolve inconsistent ledger state");
                        // Set the starting block height as the height of the ledger roots block height.
                        let mut current_block_height = latest_block_height_0;

                        // Perform all the associated storage operations as an atomic batch if it's not part of a batch yet.
                        let batch = if let Some(id) = batch {
                            id
                        } else {
                            self.ledger_roots.prepare_batch()
                        };

                        // Decrement down to the block height stored in the block heights map.
                        while current_block_height > latest_block_height_1 {
                            // Find the corresponding ledger root that was not removed.
                            let mut candidate_ledger_root = None;
                            // Attempt to find the previous ledger root corresponding to the current block height.
                            for (previous_ledger_root, block_height) in self.ledger_roots.iter() {
                                // If found, set the previous ledger root, and break.
                                if block_height == current_block_height {
                                    candidate_ledger_root = Some(previous_ledger_root);
                                    break;
                                }
                            }

                            // Update the internal state of the ledger roots, if a candidate was found.
                            if let Some(previous_ledger_root) = candidate_ledger_root {
                                self.ledger_roots.remove(&previous_ledger_root, Some(batch))?;
                                current_block_height = current_block_height.saturating_sub(1);
                            } else {
                                // Discard the in-progress batch if it's a standalone operation.
                                if !is_part_of_a_batch {
                                    self.ledger_roots.discard_batch(batch)?;
                                }

                                return Err(anyhow!(
                                    "Loaded a ledger with inconsistent state ({} != {}) (failed to automatically resolve)",
                                    current_block_height,
                                    latest_block_height_1
                                ));
                            }
                        }

                        // Execute the pending storage batch if it's a standalone operation.
                        if !is_part_of_a_batch {
                            self.ledger_roots.execute_batch(batch)?;
                        }

                        // If this is reached, the inconsistency was automatically resolved,
                        // proceed to return the new block height and continue on.
                        debug!("Successfully resolved inconsistent ledger state");
                        Ok(current_block_height)
                    } else {
                        Err(anyhow!(
                            "Loaded a ledger with inconsistent state ({} != {}) (unable to automatically resolve)",
                            latest_block_height_0,
                            latest_block_height_1
                        ))
                    }
                }
            },
            (None, None) => Ok(0u32),
            _ => Err(anyhow!("Ledger storage state is inconsistent")),
        }
    }

    /// Attempts to revert from the latest block height to the given revert block height.
    fn clear_incompatible_blocks(&self, latest_block_height: u32, revert_block_height: u32) -> Result<u32> {
        // Perform all the associated storage operations as an atomic batch.
        let batch = self.ledger_roots.prepare_batch();

        // Process the block removals.
        let mut current_block_height = latest_block_height;
        while current_block_height > revert_block_height {
            // Update the internal storage state of the ledger.
            // Ensure the block height is not the genesis block.
            if current_block_height == 0 {
                break;
            }

            // Retrieve the block hash.
            let block_hash = match self.blocks.block_heights.get(&current_block_height)? {
                Some(block_hash) => block_hash,
                None => {
                    warn!("Block {} missing from block heights map", current_block_height);
                    break;
                }
            };
            // Retrieve the block transaction IDs.
            let transaction_ids = match self.blocks.block_transactions.get(&block_hash)? {
                Some(transaction_ids) => transaction_ids,
                None => {
                    warn!("Block {} missing from block transactions map", block_hash);
                    break;
                }
            };

            // Remove the block height.
            self.blocks.block_heights.remove(&current_block_height, Some(batch))?;
            // Remove the block header.
            self.blocks.block_headers.remove(&block_hash, Some(batch))?;
            // Remove the block transactions.
            self.blocks.block_transactions.remove(&block_hash, Some(batch))?;
            // Remove the transactions.
            for transaction_ids in transaction_ids.iter() {
                self.blocks.transactions.remove_transaction(transaction_ids, Some(batch))?;
            }

            // Remove the ledger root corresponding to the current block height.
            let remove_ledger_root = self
                .ledger_roots
                .iter()
                .filter(|(_, block_height)| current_block_height == *block_height);

            for (ledger_root, _) in remove_ledger_root {
                self.ledger_roots.remove(&ledger_root, Some(batch))?;
            }

            // Decrement the current block height, and update the current block.
            current_block_height = current_block_height.saturating_sub(1);

            trace!("Ledger successfully reverted to block {}", current_block_height);
        }

        // Execute the pending storage batch.
        self.ledger_roots.execute_batch(batch)?;

        Ok(current_block_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        rocksdb::{tests::temp_dir, RocksDB},
        ReadOnly, ReadWrite, Storage,
    };
    use snarkvm::prelude::Testnet3;

    type CurrentNetwork = Testnet3;
    type A = snarkvm::circuit::AleoV0;

    #[test]
    fn test_open_ledger_state_reader() {
        let dir = temp_dir();
        {
            let _block_state =
                LedgerState::<CurrentNetwork, ReadWrite, A>::open_writer::<RocksDB, _>(&dir).expect("Failed to open ledger state");
        }

        let _block_state =
            LedgerState::<CurrentNetwork, ReadWrite, A>::open_reader::<RocksDB, _>(dir).expect("Failed to open ledger state");
    }

    #[test]
    fn test_open_ledger_state_writer() {
        let _block_state =
            LedgerState::<CurrentNetwork, ReadWrite, A>::open_writer::<RocksDB, _>(temp_dir()).expect("Failed to open ledger state");
    }
}
