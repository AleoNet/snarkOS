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

use anyhow::*;
use std::sync::Arc;

use crate::{Digest, FixMode, SerialBlock, SerialBlockHeader, SerialRecord, SerialTransaction, TransactionLocation};

/// Current state of a block in storage
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockStatus {
    /// Block not known/not found
    Unknown,
    /// Block on canon chain @ height
    Committed(usize),
    /// Block known, but not in canon chain
    Uncommitted,
}

pub struct ForkPath {
    /// Index of the canon block this fork is based on.
    pub base_index: u32,
    /// Set of digests from `base_index`'s corresponding block to the target block
    pub path: Vec<Digest>,
}

pub enum ForkDescription {
    /// A valid fork path was found from a canon block
    Path(ForkPath),
    /// There might be a valid fork path, but it was too long to tell
    TooLong,
    /// The block never found a canon ancestor
    Orphan,
}

pub struct CanonData {
    /// Current block height of canon
    pub block_height: usize,
    /// Current hash of canon block
    pub hash: Digest,
}

#[derive(Debug)]
pub enum BlockFilter {
    CanonOnly,
    NonCanonOnly,
    All,
}

/// An application level storage interface
/// Requires atomicity within each method implementation, but doesn't require any kind of consistency between invocations other than call-order enforcement.
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Inserts a block into storage, not committing it.
    async fn insert_block(&self, block: &SerialBlock) -> Result<()>;

    /// Deletes a block from storage, including any associated data. Must not be called on a committed block.
    async fn delete_block(&self, hash: &Digest) -> Result<()>;

    /// Gets a hash for a canon block number, if it exists
    async fn get_block_hash(&self, block_num: u32) -> Result<Option<Digest>>;

    /// Gets a block header for a given hash
    async fn get_block_header(&self, hash: &Digest) -> Result<SerialBlockHeader>;

    /// Gets a block status for a given hash
    async fn get_block_state(&self, hash: &Digest) -> Result<BlockStatus>;

    /// Bulk operation of `Storage::get_block_state`, gets many block statuses for many hashes.
    async fn get_block_states(&self, hashes: &[Digest]) -> Result<Vec<BlockStatus>>;

    /// Gets a block header and transaction blob for a given hash.
    async fn get_block(&self, hash: &Digest) -> Result<SerialBlock>;

    /// Finds a fork path from any applicable canon node within `oldest_fork_threshold` to `hash`.
    async fn get_fork_path(&self, hash: &Digest, oldest_fork_threshold: usize) -> Result<ForkDescription>;

    /// Commits a block into canon.
    async fn commit_block(&self, hash: &Digest, digest: Digest) -> Result<BlockStatus>;

    /// Decommits a block and all descendent blocks, returning them in ascending order
    async fn decommit_blocks(&self, hash: &Digest) -> Result<Vec<SerialBlock>>;

    /// Gets the current canon state of storage
    async fn canon(&self) -> Result<CanonData>;

    /// Gets the longest, committed or uncommitted, chain of blocks originating from `block_hash`, including `block_hash`.
    async fn longest_child_path(&self, block_hash: &Digest) -> Result<Vec<Digest>>;

    /// Gets a series of hashes used for relaying current block sync state.
    async fn get_block_locator_hashes(&self) -> Result<Vec<Digest>>;

    /// Find hashes to provide for a syncing node given `block_locator_hashes`.
    async fn find_sync_blocks(&self, block_locator_hashes: &[Digest], block_count: usize) -> Result<Vec<Digest>>;

    /// Gets the block and transaction index of a transaction in a block.
    async fn get_transaction_location(&self, transaction_id: Digest) -> Result<Option<TransactionLocation>>;

    /// Gets a transaction from a transaction id
    async fn get_transaction(&self, transaction_id: Digest) -> Result<SerialTransaction> {
        let location = self
            .get_transaction_location(transaction_id)
            .await?
            .ok_or_else(|| anyhow!("transaction not found"))?;
        let block = self.get_block(&location.block_hash).await?;
        if let Some(transaction) = block.transactions.get(location.index as usize) {
            Ok(transaction.clone())
        } else {
            Err(anyhow!("missing transaction in block"))
        }
    }

    // miner convenience record management functions

    /// Gets a list of stored record commitments subject to `limit`.
    async fn get_record_commitments(&self, limit: Option<usize>) -> Result<Vec<Digest>>;

    /// Gets a record blob given a commitment.
    async fn get_record(&self, commitment: Digest) -> Result<Option<SerialRecord>>;

    /// Stores a series of new record blobs and their commitments.
    async fn store_records(&self, records: &[SerialRecord]) -> Result<()>;

    /// Gets all known commitments for canon chain in block-number ascending order
    async fn get_commitments(&self) -> Result<Vec<Digest>>;

    /// Gets all known serial numbers for canon chain in block-number ascending order
    async fn get_serial_numbers(&self) -> Result<Vec<Digest>>;

    /// Gets all known memos for canon chain in block-number ascending order
    async fn get_memos(&self) -> Result<Vec<Digest>>;

    /// Gets all known ledger digests for canon chain in block-number ascending order
    async fn get_ledger_digests(&self) -> Result<Vec<Digest>>;

    /// Resets stored ledger state. A maintenance function, not intended for general use.
    async fn reset_ledger(
        &self,
        commitments: Vec<Digest>,
        serial_numbers: Vec<Digest>,
        memos: Vec<Digest>,
        digests: Vec<Digest>,
    ) -> Result<()>;

    /// Gets a dump of all stored canon blocks, in block-number ascending order. A maintenance function, not intended for general use.
    async fn get_canon_blocks(&self, limit: Option<u32>) -> Result<Vec<SerialBlock>>;

    /// Similar to `Storage::get_canon_blocks`, gets hashes of all blocks subject to `filter` and `limit` in block-number ascending order. A maintenance function, not intended for general use.
    async fn get_block_hashes(&self, limit: Option<u32>, filter: BlockFilter) -> Result<Vec<Digest>>;

    /// Performs low-level storage validation; it's mostly intended for test purposes, as there is a lower level `KeyValueStorage` interface available outside of them.
    async fn validate(&self, limit: Option<u32>, fix_mode: FixMode) -> bool;
}

/// A wrapper over storage implementations
pub type DynStorage = Arc<dyn Storage>;
