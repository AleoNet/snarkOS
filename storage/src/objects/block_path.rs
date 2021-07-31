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

use crate::Ledger;
use snarkvm::{
    dpc::Parameters,
    ledger::{BlockError, BlockHeader, BlockHeaderHash, LedgerScheme, Storage, StorageError},
};

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

    /// The accumulated difficulty targets for all block headers in the sidechain.
    pub aggregate_difficulty: u128,
}

impl<C: Parameters, S: Storage> Ledger<C, S> {
    /// Get the block's path/origin.
    pub fn get_block_path(&self, block_header: &BlockHeader) -> Result<BlockPath, StorageError> {
        let block_hash = block_header.get_hash();

        // The given block header already exists
        if self.is_canon(&block_hash) {
            return Ok(BlockPath::ExistingBlock);
        }

        // The given block header is valid on the canon chain
        if self.latest_block()?.header.get_hash() == block_header.previous_block_hash {
            return Ok(BlockPath::CanonChain(self.block_height()));
        }

        let mut side_chain_path = vec![];
        let mut side_chain_diff = block_header.difficulty_target as u128;
        let mut parent_hash = block_header.previous_block_hash.clone();

        // Find the sidechain path (with a maximum size of OLDEST_FORK_THRESHOLD)
        for _ in 0..=OLDEST_FORK_THRESHOLD {
            // check if the part is part of the canon chain
            match &self.get_block_number(&parent_hash) {
                // This is a canon parent
                Ok(block_num) => {
                    // Add the children from the latest block
                    let longest_path = self.longest_child_path(block_hash)?;

                    // Add all the difficulty targets associated with the longest_path,
                    // skipping the first element (which is the hash associated to
                    // `block_header`).
                    for hash in longest_path.iter().skip(1) {
                        let block_header = self.get_block_header(hash)?;
                        side_chain_diff += block_header.difficulty_target as u128;
                    }

                    side_chain_path.extend(longest_path);

                    return Ok(BlockPath::SideChain(SideChainPath {
                        shared_block_number: *block_num,
                        new_block_number: block_num + side_chain_path.len() as u32,
                        path: side_chain_path,
                        aggregate_difficulty: side_chain_diff,
                    }));
                }
                // Add to the side_chain_path
                Err(_) => {
                    side_chain_path.insert(0, parent_hash.clone());
                    let parent_header = self.get_block_header(&parent_hash)?;
                    side_chain_diff += parent_header.difficulty_target as u128;
                    parent_hash = parent_header.previous_block_hash;
                }
            }
        }

        Err(StorageError::BlockError(BlockError::IrrelevantBlock(
            block_hash.to_string(),
        )))
    }

    /// Returns the path length and the longest path of children from the given block header
    pub fn longest_child_path(&self, block_hash: BlockHeaderHash) -> Result<Vec<BlockHeaderHash>, StorageError> {
        let mut round = vec![vec![block_hash]];
        let mut next_round = vec![];
        loop {
            for path in &round {
                let children = self.get_child_block_hashes(path.last().unwrap())?;
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
}
