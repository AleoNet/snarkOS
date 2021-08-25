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

use std::collections::VecDeque;

use anyhow::*;
use tracing::{debug, trace};

#[cfg(feature = "test")]
use crate::key_value::KeyValueColumn;
use crate::{
    BlockFilter,
    BlockStatus,
    CanonData,
    Digest,
    DigestTree,
    FixMode,
    ForkDescription,
    ForkPath,
    SerialBlock,
    SerialBlockHeader,
    SerialRecord,
    SerialTransaction,
    TransactionLocation,
    ValidatorError,
};

/// An application level storage interface
/// Synchronous version of [`crate::Storage`]
pub trait SyncStorage {
    /// Initialization function that will be called before any other functions
    fn init(&mut self) -> Result<()>;

    /// Runs a function within a write-cancellable context
    fn transact<T, F: FnOnce(&mut Self) -> Result<T>>(&mut self, func: F) -> Result<T>;

    /// Inserts a block into storage, not committing it.
    fn insert_block(&mut self, block: &SerialBlock) -> Result<()>;

    /// Deletes a block from storage, including any associated data. Must not be called on a committed block.
    fn delete_block(&mut self, hash: &Digest) -> Result<()>;

    /// Gets a hash for a canon block number, if it exists
    fn get_block_hash(&mut self, block_num: u32) -> Result<Option<Digest>>;

    /// Gets a hash for a canon block number, if it exists, otherwise it errors out
    fn get_block_hash_guarded(&mut self, block_num: u32) -> Result<Digest> {
        self.get_block_hash(block_num)?
            .ok_or_else(|| anyhow!("missing canon hash"))
    }

    /// Gets a block header for a given hash
    fn get_block_header(&mut self, hash: &Digest) -> Result<SerialBlockHeader>;

    /// Gets a block status for a given hash
    fn get_block_state(&mut self, hash: &Digest) -> Result<BlockStatus>;

    /// Bulk operation of `Storage::get_block_state`, gets many block statuses for many hashes.
    fn get_block_states(&mut self, hashes: &[Digest]) -> Result<Vec<BlockStatus>>;

    /// Gets a block header and transaction blob for a given hash.
    fn get_block(&mut self, hash: &Digest) -> Result<SerialBlock>;

    /// Finds a fork path from any applicable canon node within `oldest_fork_threshold` to `hash`.
    fn get_fork_path(&mut self, hash: &Digest, oldest_fork_threshold: usize) -> Result<ForkDescription> {
        let mut side_chain_path = VecDeque::new();
        let header = self.get_block_header(hash)?;
        let canon_height = self.canon_height()?;
        let mut parent_hash = header.previous_block_hash;
        for _ in 0..=oldest_fork_threshold {
            // check if the part is part of the canon chain
            match self.get_block_state(&parent_hash)? {
                // This is a canon parent
                BlockStatus::Committed(block_num) => {
                    // Add the children from the latest block
                    if canon_height as usize - block_num > oldest_fork_threshold {
                        debug!("exceeded maximum fork length in extended path");
                        return Ok(ForkDescription::TooLong);
                    }
                    let longest_path = self.longest_child_path(hash)?;
                    debug!("longest child path terminating in {:?}", longest_path.last());
                    side_chain_path.extend(longest_path);
                    return Ok(ForkDescription::Path(ForkPath {
                        base_index: block_num as u32,
                        path: side_chain_path.into(),
                    }));
                }
                // Add to the side_chain_path
                BlockStatus::Uncommitted => {
                    side_chain_path.push_front(parent_hash.clone());
                    parent_hash = self.get_block_header(&parent_hash)?.previous_block_hash;
                }
                BlockStatus::Unknown => {
                    return Ok(ForkDescription::Orphan);
                }
            }
        }
        Ok(ForkDescription::TooLong)
    }

    /// Commits a block into canon.
    fn commit_block(&mut self, hash: &Digest, digest: &Digest) -> Result<BlockStatus>;

    /// Decommits a block and all descendent blocks, returning them in ascending order
    fn decommit_blocks(&mut self, hash: &Digest) -> Result<Vec<SerialBlock>>;

    /// Gets the current canon height of storage
    fn canon_height(&mut self) -> Result<u32>;

    /// Gets the current canon state of storage
    fn canon(&mut self) -> Result<CanonData>;

    /// Gets the longest, committed or uncommitted, chain of blocks originating from `block_hash`, including `block_hash`.
    fn longest_child_path(&mut self, block_hash: &Digest) -> Result<Vec<Digest>> {
        let mut round = vec![vec![block_hash.clone()]];
        let mut next_round = vec![];
        loop {
            for path in &round {
                let children = self.get_block_children(path.last().unwrap())?;
                next_round.extend(children.into_iter().map(|x| {
                    let mut path = path.clone();
                    path.push(x);
                    path
                }));
            }
            if next_round.is_empty() {
                break;
            }
            round = next_round;
            next_round = vec![];
        }

        Ok(round.into_iter().max_by_key(|x| x.len()).unwrap())
    }

    /// Gets a tree structure representing all the descendents of [`block_hash`]
    fn get_block_digest_tree(&mut self, block_hash: &Digest) -> Result<DigestTree> {
        let children = self.get_block_children(block_hash)?;
        if children.len() == 1 {
            return Ok(DigestTree::Leaf(block_hash.clone()));
        }
        let mut out_children = Vec::with_capacity(children.len());
        let mut longest_tree_len = 0usize;
        for child in children {
            let subtree = self.get_block_digest_tree(&child)?;
            let len = subtree.longest_length() + 1;
            if len > longest_tree_len {
                longest_tree_len = len;
            }
            out_children.push(subtree);
        }
        Ok(DigestTree::Node(block_hash.clone(), out_children, longest_tree_len))
    }

    /// Gets the immediate children of `block_hash`.
    fn get_block_children(&mut self, block_hash: &Digest) -> Result<Vec<Digest>>;

    /// Gets a series of hashes used for relaying current block sync state.
    fn get_block_locator_hashes(
        &mut self,
        points_of_interest: Vec<Digest>,
        oldest_fork_threshold: usize,
    ) -> Result<Vec<Digest>> {
        let canon = self.canon()?;
        let target_height = canon.block_height as u32;

        // The number of locator hashes left to obtain; accounts for the genesis block.
        let mut num_locator_hashes = std::cmp::min(crate::NUM_LOCATOR_HASHES - 1, target_height);

        // The output list of block locator hashes.
        let mut block_locator_hashes = Vec::with_capacity(num_locator_hashes as usize + points_of_interest.len());

        for hash in points_of_interest {
            trace!("block locator hash -- interesting: block# none: {}", hash);
            block_locator_hashes.push(hash);
        }

        // The index of the current block for which a locator hash is obtained.
        let mut hash_index = target_height;

        // The number of top blocks to provide locator hashes for.
        let num_top_blocks = std::cmp::min(10, num_locator_hashes);

        for _ in 0..num_top_blocks {
            let hash = self.get_block_hash_guarded(hash_index)?;
            trace!("block locator hash -- top: block# {}: {}", hash_index, hash);
            block_locator_hashes.push(hash);
            hash_index -= 1; // safe; num_top_blocks is never higher than the height
        }

        num_locator_hashes -= num_top_blocks;
        if num_locator_hashes == 0 {
            let hash = self.get_block_hash_guarded(0)?;
            trace!("block locator hash -- genesis: block# {}: {}", 0, hash);
            block_locator_hashes.push(hash);
            return Ok(block_locator_hashes);
        }

        // Calculate the average distance between block hashes based on the desired number of locator hashes.
        let mut proportional_step =
            (hash_index.min(oldest_fork_threshold as u32) / num_locator_hashes).min(crate::NUM_LOCATOR_HASHES - 1);

        // Provide hashes of blocks with indices descending quadratically while the quadratic step distance is
        // lower or close to the proportional step distance.
        let num_quadratic_steps = (proportional_step as f32).log2() as u32;

        // The remaining hashes should have a proportional index distance between them.
        let num_proportional_steps = num_locator_hashes - num_quadratic_steps;

        // Obtain a few hashes increasing the distance quadratically.
        let mut quadratic_step = 2; // the size of the first quadratic step
        for _ in 0..num_quadratic_steps {
            let hash = self.get_block_hash_guarded(hash_index)?;
            trace!("block locator hash -- quadratic: block# {}: {}", hash_index, hash);
            block_locator_hashes.push(hash);
            hash_index = hash_index.saturating_sub(quadratic_step);
            quadratic_step *= 2;
        }

        // Update the size of the proportional step so that the hashes of the remaining blocks have the same distance
        // between one another.
        proportional_step =
            (hash_index.min(oldest_fork_threshold as u32) / num_locator_hashes).min(crate::NUM_LOCATOR_HASHES - 1);

        // Tweak: in order to avoid "jumping" by too many indices with the last step,
        // increase the value of each step by 1 if the last step is too large. This
        // can result in the final number of locator hashes being a bit lower, but
        // it's preferable to having a large gap between values.
        if hash_index - proportional_step * num_proportional_steps > 2 * proportional_step {
            proportional_step += 1;
        }

        // Obtain the rest of hashes with a proportional distance between them.
        for _ in 0..num_proportional_steps {
            let hash = self.get_block_hash_guarded(hash_index)?;
            trace!("block locator hash -- proportional: block# {}: {}", hash_index, hash);
            block_locator_hashes.push(hash);
            if hash_index == 0 {
                return Ok(block_locator_hashes);
            }
            hash_index = hash_index.saturating_sub(proportional_step);
        }

        let hash = self.get_block_hash_guarded(0)?;
        trace!("block locator hash -- genesis: block# {}: {}", 0, hash);
        block_locator_hashes.push(hash);

        Ok(block_locator_hashes)
    }

    /// Find hashes to provide for a syncing node given `block_locator_hashes`.
    fn find_sync_blocks(&mut self, block_locator_hashes: Vec<Digest>, block_count: usize) -> Result<Vec<Digest>> {
        let mut min_hash = None;
        for hash in block_locator_hashes.iter() {
            if matches!(self.get_block_state(hash)?, BlockStatus::Committed(_)) {
                min_hash = Some(hash.clone());
                break;
            }
        }
        let min_height = if let Some(min_hash) = min_hash {
            let min_height = self.get_block_state(&min_hash)?;
            match min_height {
                BlockStatus::Committed(n) => n + 1,
                _ => return Err(anyhow!("illegal block state")),
            }
        } else {
            0
        };
        let mut max_height = min_height + block_count;
        let canon = self.canon()?;
        if canon.block_height < max_height {
            max_height = canon.block_height;
        }
        let mut hashes = vec![];
        for i in min_height..=max_height {
            hashes.push(self.get_block_hash_guarded(i as u32)?);
        }
        Ok(hashes)
    }

    /// Gets the block and transaction index of a transaction in a block.
    fn get_transaction_location(&mut self, transaction_id: &Digest) -> Result<Option<TransactionLocation>>;

    /// Gets a transaction from a transaction id
    fn get_transaction(&mut self, transaction_id: &Digest) -> Result<SerialTransaction>;

    /// Stores the "pre-genesis" digest; only applicable to the genesis block txs.
    fn store_init_digest(&mut self, digest: &Digest) -> Result<()>;

    // miner convenience record management functions

    /// Gets a list of stored record commitments subject to `limit`.
    fn get_record_commitments(&mut self, limit: Option<usize>) -> Result<Vec<Digest>>;

    /// Gets a record blob given a commitment.
    fn get_record(&mut self, commitment: &Digest) -> Result<Option<SerialRecord>>;

    /// Stores a series of new record blobs and their commitments.
    fn store_records(&mut self, records: &[SerialRecord]) -> Result<()>;

    /// Gets all known commitments for canon chain in block-number ascending order
    fn get_commitments(&mut self) -> Result<Vec<Digest>>;

    /// Gets all known serial numbers for canon chain in block-number ascending order
    fn get_serial_numbers(&mut self) -> Result<Vec<Digest>>;

    /// Gets all known memos for canon chain in block-number ascending order
    fn get_memos(&mut self) -> Result<Vec<Digest>>;

    /// Gets all known ledger digests for canon chain in block-number ascending order
    fn get_ledger_digests(&mut self) -> Result<Vec<Digest>>;

    /// Resets stored ledger state. A maintenance function, not intended for general use.
    fn reset_ledger(
        &mut self,
        commitments: Vec<Digest>,
        serial_numbers: Vec<Digest>,
        memos: Vec<Digest>,
        digests: Vec<Digest>,
    ) -> Result<()>;

    /// Gets a dump of all stored canon blocks, in block-number ascending order. A maintenance function, not intended for general use.
    fn get_canon_blocks(&mut self, limit: Option<u32>) -> Result<Vec<SerialBlock>>;

    /// Similar to `Storage::get_canon_blocks`, gets hashes of all blocks subject to `filter` and `limit` in filter-defined order. A maintenance function, not intended for general use.
    fn get_block_hashes(&mut self, limit: Option<u32>, filter: BlockFilter) -> Result<Vec<Digest>>;

    /// Performs low-level storage validation; it's mostly intended for test purposes, as there is a lower level `KeyValueStorage` interface available outside of them.
    fn validate(&mut self, limit: Option<u32>, fix_mode: FixMode) -> Vec<ValidatorError>;

    /// Stores the given key+value pair in the given column.
    #[cfg(feature = "test")]
    fn store_item(&mut self, col: KeyValueColumn, key: Vec<u8>, value: Vec<u8>) -> Result<()>;

    /// Removes the given key and its corresponding value from the given column.
    #[cfg(feature = "test")]
    fn delete_item(&mut self, col: KeyValueColumn, key: Vec<u8>) -> Result<()>;
}
