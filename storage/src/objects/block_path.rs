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

use crate::error::StorageError;
use crate::Ledger;
use snarkvm_objects::BlockError;
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_objects::Transaction;
use snarkvm_objects::BlockHeader;
use snarkvm_objects::BlockHeaderHash;

const OLDEST_FORK_THRESHOLD: u32 = 1024;

#[derive(Clone, Debug)]
pub enum BlockPath {
    ExistingBlock,
    CanonChain(u32),
    SideChain(SideChainPath),
}

#[derive(Clone, Debug)]
pub struct SideChainPath {
    /// Latest block number before diverging from the canon chain.
    pub shared_block_number: u32,

    /// New block number
    pub new_block_number: u32,

    /// Path of block hashes from the shared block to the latest diverging block (oldest first).
    pub path: Vec<BlockHeaderHash>,
}

impl<T: Transaction, P: LoadableMerkleParameters> Ledger<T, P> {
    /// Get the block's path/origin.
    pub fn get_block_path(&self, block_header: &BlockHeader) -> Result<BlockPath, StorageError> {
        let block_hash = block_header.get_hash();

        // The given block header already exists
        if self.block_hash_exists(&block_hash) {
            return Ok(BlockPath::ExistingBlock);
        }

        // The given block header is valid on the canon chain
        if self.get_latest_block()?.header.get_hash() == block_header.previous_block_hash {
            return Ok(BlockPath::CanonChain(self.get_current_block_height() + 1));
        }

        let mut side_chain_path = vec![];
        let mut parent_hash = block_header.previous_block_hash.clone();

        // Find the sidechain path (with a maximum size of OLDEST_FORK_THRESHOLD)
        for _ in 0..=OLDEST_FORK_THRESHOLD {
            // check if the part is part of the canon chain
            match &self.get_block_number(&parent_hash) {
                // This is a canon parent
                Ok(block_num) => {
                    // Add the children from the latest block

                    let longest_path = self.longest_child_path(block_hash)?;

                    side_chain_path.extend(longest_path.1);

                    return Ok(BlockPath::SideChain(SideChainPath {
                        shared_block_number: *block_num,
                        new_block_number: block_num + side_chain_path.len() as u32,
                        path: side_chain_path,
                    }));
                }
                // Add to the side_chain_path
                Err(_) => {
                    side_chain_path.insert(0, parent_hash.clone());
                    parent_hash = self.get_block_header(&parent_hash)?.previous_block_hash;
                }
            }
        }

        Err(StorageError::BlockError(BlockError::IrrelevantBlock(
            block_hash.to_string(),
        )))
    }

    /// Returns the path length and the longest path of children from the given block header
    pub fn longest_child_path(
        &self,
        block_hash: BlockHeaderHash,
    ) -> Result<(usize, Vec<BlockHeaderHash>), StorageError> {
        let children = self.get_child_block_hashes(&block_hash)?;

        let mut final_path = vec![block_hash];

        if children.is_empty() {
            Ok((1, final_path))
        } else {
            let mut paths = Vec::with_capacity(children.len());
            for child in children {
                paths.push(self.longest_child_path(child)?);
            }

            match paths.iter().max_by_key(|x| x.0) {
                Some((longest_child_path_length, longest_child_path)) => {
                    final_path.extend(longest_child_path.clone());

                    Ok((*longest_child_path_length + 1, final_path))
                }
                None => Ok((1, final_path)),
            }
        }
    }
}
