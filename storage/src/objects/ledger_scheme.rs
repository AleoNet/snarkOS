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

use crate::*;
use snarkvm::{
    algorithms::merkle_tree::*,
    dpc::{Parameters, RecordCommitmentTree, RecordSerialNumberTree, Transaction},
    ledger::{Block, BlockHeaderHash, BlockScheme, LedgerError, LedgerScheme, Storage, StorageError},
    utilities::{to_bytes_le, FromBytes, ToBytes},
};

use anyhow::Result;
use arc_swap::ArcSwap;
use std::{
    fs,
    path::Path,
    sync::{atomic::Ordering, Arc},
};

impl<C: Parameters, S: Storage> LedgerScheme<C> for Ledger<C, S> {
    type Block = Block<Transaction<C>>;

    /// Instantiates a new ledger with a genesis block.
    fn new(path: Option<&Path>, genesis_block: Self::Block) -> Result<Self> {
        // Ensure the given block is a genesis block.
        if !genesis_block.header().is_genesis() {
            return Err(LedgerError::InvalidGenesisBlockHeader.into());
        }

        let storage = if let Some(path) = path {
            fs::create_dir_all(&path).map_err(|err| LedgerError::Message(err.to_string()))?;

            S::open(Some(path), None)
        } else {
            S::open(None, None) // this must mean we're using an in-memory storage
        }?;

        if let Some(block_num) = storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())? {
            if bytes_to_u32(&block_num) != 0 {
                return Err(LedgerError::ExistingDatabase.into());
            }
        }

        let leaves: &[[u8; 32]] = &[];
        let parameters = Arc::new(C::record_commitment_tree_parameters().clone());
        let empty_cm_merkle_tree = MerkleTree::new(parameters, leaves)?;

        let ledger = Self {
            current_block_height: Default::default(),
            storage,
            cm_merkle_tree: ArcSwap::new(Arc::new(empty_cm_merkle_tree)),
        };

        debug_assert_eq!(ledger.block_height(), 0, "Uninitialized ledger block height must be 0");
        ledger.insert_and_commit(&genesis_block)?;

        Ok(ledger)
    }

    /// Returns the number of blocks including the genesis block
    fn block_height(&self) -> u32 {
        self.current_block_height.load(Ordering::SeqCst)
    }

    /// Returns the latest block in the ledger.
    fn latest_block(&self) -> Result<Block<Transaction<C>>> {
        let block_hash = self.get_block_hash(self.block_height())?;
        self.get_block(&block_hash)
    }

    /// Returns the block given the block hash.
    fn get_block(&self, block_hash: &BlockHeaderHash) -> Result<Block<Transaction<C>>> {
        Ok(Block {
            header: self.get_block_header(block_hash)?,
            transactions: self.get_block_transactions(block_hash)?,
        })
    }

    /// Returns the block hash given a block number.
    fn get_block_hash(&self, block_number: u32) -> Result<BlockHeaderHash> {
        match self.storage.get(COL_BLOCK_LOCATOR, &block_number.to_le_bytes())? {
            Some(block_header_hash) => Ok(BlockHeaderHash::new(block_header_hash)),
            None => Err(StorageError::MissingBlockHash(block_number).into()),
        }
    }

    /// Returns the block number given a block hash.
    fn get_block_number(&self, block_hash: &BlockHeaderHash) -> Result<u32> {
        match self.storage.get(COL_BLOCK_LOCATOR, &block_hash.0)? {
            Some(block_num_bytes) => Ok(bytes_to_u32(&block_num_bytes)),
            None => Err(StorageError::MissingBlockNumber(block_hash.to_string()).into()),
        }
    }

    /// Returns true if the given block hash exists in the ledger.
    fn contains_block_hash(&self, block_hash: &BlockHeaderHash) -> bool {
        self.get_block_header(block_hash).is_ok()
    }
}

impl<C: Parameters, S: Storage> RecordCommitmentTree<C> for Ledger<C, S> {
    /// Return the latest state root of the record commitment tree.
    fn latest_digest(&self) -> Result<MerkleTreeDigest<C::RecordCommitmentTreeParameters>> {
        let digest = match self.storage.get(COL_META, KEY_CURR_DIGEST.as_bytes())? {
            Some(current_digest) => current_digest,
            None => to_bytes_le![self.cm_merkle_tree.load().root()]?,
        };
        Ok(FromBytes::read_le(digest.as_slice())?)
    }

    /// Check that st_{ts} is a valid digest for some (past) ledger state.
    fn is_valid_digest(&self, digest: &MerkleTreeDigest<C::RecordCommitmentTreeParameters>) -> bool {
        self.storage.exists(COL_DIGEST, &digest.to_bytes_le().unwrap())
    }

    /// Returns true if the given commitment exists in the ledger.
    fn contains_commitment(&self, commitment: &C::RecordCommitment) -> bool {
        self.storage.exists(COL_COMMITMENT, &commitment.to_bytes_le().unwrap())
    }

    /// Returns the Merkle path to the latest ledger digest
    /// for a given commitment, if it exists in the ledger.
    fn prove_cm(&self, cm: &C::RecordCommitment) -> Result<MerklePath<C::RecordCommitmentTreeParameters>> {
        let cm_index = self
            .get_cm_index(&cm.to_bytes_le()?)?
            .ok_or(LedgerError::InvalidCmIndex)?;
        let result = self.cm_merkle_tree.load().generate_proof(cm_index, cm)?;

        Ok(result)
    }
}

impl<C: Parameters, S: Storage> RecordSerialNumberTree<C> for Ledger<C, S> {
    /// Returns true if the given serial number exists in the ledger.
    fn contains_serial_number(&self, serial_number: &C::AccountSignaturePublicKey) -> bool {
        self.storage
            .exists(COL_SERIAL_NUMBER, &serial_number.to_bytes_le().unwrap())
    }
}
