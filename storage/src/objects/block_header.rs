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

use crate::{Ledger, COL_BLOCK_HEADER};
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_dpc::{errors::StorageError, Block, BlockHeader, BlockHeaderHash, Storage, TransactionScheme};
use snarkvm_utilities::FromBytes;

impl<T: TransactionScheme, P: LoadableMerkleParameters, S: Storage> Ledger<T, P, S> {
    /// Returns true if the block for the given block header hash exists.
    pub fn block_hash_exists(&self, block_hash: &BlockHeaderHash) -> bool {
        if self.is_empty() {
            return false;
        }

        self.get_block_header(block_hash).is_ok()
    }

    /// Get a block header given the block hash.
    pub fn get_block_header(&self, block_hash: &BlockHeaderHash) -> Result<BlockHeader, StorageError> {
        match self.storage.get(COL_BLOCK_HEADER, &block_hash.0)? {
            Some(block_header_bytes) => Ok(BlockHeader::read_le(&block_header_bytes[..])?),
            None => Err(StorageError::MissingBlockHeader(block_hash.to_string())),
        }
    }

    /// Returns true if the block corresponding to this block's previous_block_hash exists.
    pub fn previous_block_hash_exists(&self, block: &Block<T>) -> bool {
        self.block_hash_exists(&block.header.previous_block_hash)
    }

    /// Returns the latest shared block header hash.
    /// If the block locator hashes are for a side chain, returns the common point of fork.
    /// If the block locator hashes are for the canon chain, returns the latest block header hash.
    pub fn get_latest_shared_hash(
        &self,
        block_locator_hashes: Vec<BlockHeaderHash>,
    ) -> Result<BlockHeaderHash, StorageError> {
        for block_hash in block_locator_hashes {
            if self.is_canon(&block_hash) {
                return Ok(block_hash);
            }
        }

        self.get_block_hash(0)
    }

    /// Returns a list of block locator hashes. The purpose of this method is to detect
    /// wrong branches in the caller's canon chain.
    pub fn get_block_locator_hashes(&self) -> Result<Vec<BlockHeaderHash>, StorageError> {
        let block_height = self.get_current_block_height();

        // The number of locator hashes left to obtain; accounts for the genesis block.
        let mut num_locator_hashes = std::cmp::min(crate::NUM_LOCATOR_HASHES - 1, block_height);

        // The output list of block locator hashes.
        let mut block_locator_hashes = Vec::with_capacity(num_locator_hashes as usize);

        // The index of the current block for which a locator hash is obtained.
        let mut hash_index = block_height;

        // The number of top blocks to provide locator hashes for.
        let num_top_blocks = std::cmp::min(10, num_locator_hashes);

        for _ in 0..num_top_blocks {
            block_locator_hashes.push(self.get_block_hash(hash_index)?);
            hash_index -= 1; // safe; num_top_blocks is never higher than the height
        }

        num_locator_hashes -= num_top_blocks;
        if num_locator_hashes == 0 {
            block_locator_hashes.push(self.get_block_hash(0)?);
            return Ok(block_locator_hashes);
        }

        // Calculate the average distance between block hashes based on the desired number of locator hashes.
        let mut proportional_step = hash_index / num_locator_hashes;

        // Provide hashes of blocks with indices descending quadratically while the quadratic step distance is
        // lower or close to the proportional step distance.
        let num_quadratic_steps = (proportional_step as f32).log2() as u32;

        // The remaining hashes should have a proportional index distance between them.
        let num_proportional_steps = num_locator_hashes - num_quadratic_steps;

        // Obtain a few hashes increasing the distance quadratically.
        let mut quadratic_step = 4; // the size of the first quadratic step
        for _ in 0..num_quadratic_steps {
            block_locator_hashes.push(self.get_block_hash(hash_index)?);
            hash_index = hash_index.saturating_sub(quadratic_step);
            quadratic_step *= 2;
        }

        // Update the size of the proportional step so that the hashes of the remaining blocks have the same distance
        // between one another.
        proportional_step = hash_index / num_proportional_steps;

        // Tweak: in order to avoid "jumping" by too many indices with the last step,
        // increase the value of each step by 1 if the last step is too large. This
        // can result in the final number of locator hashes being a bit lower, but
        // it's preferable to having a large gap between values.
        if hash_index - proportional_step * num_proportional_steps > 2 * proportional_step {
            proportional_step += 1;
        }

        // Obtain the rest of hashes with a proportional distance between them.
        for _ in 0..num_proportional_steps {
            block_locator_hashes.push(self.get_block_hash(hash_index)?);
            if hash_index == 0 {
                return Ok(block_locator_hashes);
            }
            hash_index = hash_index.saturating_sub(proportional_step);
        }

        block_locator_hashes.push(self.get_block_hash(0)?);

        Ok(block_locator_hashes)
    }
}
