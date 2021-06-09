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
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_dpc::{Storage, TransactionScheme};

use tracing::*;

impl<T: TransactionScheme, P: LoadableMerkleParameters, S: Storage> Ledger<T, P, S> {
    /// Performs a check of all the stored blocks and their relationships between one another; starts at the
    /// current block height and goes down until the genesis block, making sure that the block-related data
    /// stored in the database is coherent. The optional limit restricts the number of blocks to check, as
    /// it is likely that any issues are applicable only to the last few blocks.
    pub fn validate(&self, mut limit: Option<usize>) -> bool {
        info!("Validating the storage...");

        let mut is_valid = true;

        if limit == Some(0) {
            info!("The limit of blocks to validate is 0; nothing to check.");

            return is_valid;
        }

        let mut current_height = self.get_current_block_height();

        if current_height == 0 {
            info!("Only the genesis block is currently available; nothing to check.");

            return is_valid;
        }

        debug!("The block height is {}", current_height);

        match self.get_best_block_number() {
            Err(_) => {
                is_valid = false;
                error!("Can't obtain the best block number from storage!");
            }
            Ok(number) => {
                // Initial block height comes from KEY_BEST_BLOCK_NUMBER.
                if number != current_height {
                    is_valid = false;
                    error!("Current best block number doesn't match the block height!");
                }
            }
        }

        let mut true_height_mismatch = false;

        // Current block is found by COL_BLOCK_LOCATOR, as it should have been committed (i.e. be canon).
        let mut current_block = self.get_block_from_block_number(current_height);
        while let Err(e) = current_block {
            is_valid = false;
            error!(
                "Couldn't find the latest block (height {}): {}! Trying a lower height next.",
                current_height, e
            );

            true_height_mismatch = true;

            current_height -= 1;

            if let Some(ref mut limit) = limit {
                *limit -= 1;
                if *limit == 0 {
                    info!("Specified block limit reached; the check is complete.");

                    return is_valid;
                }
            }

            current_block = self.get_block_from_block_number(current_height);
        }

        let mut current_block = match current_block {
            Ok(block) => block,
            Err(e) => {
                error!("Couldn't even obtain the genesis block by height 0: {}!", e);
                error!("The storage is invalid!");

                return false;
            }
        };

        if true_height_mismatch {
            debug!("The true block height is {}", current_height);
        }

        let mut current_hash = current_block.header.get_hash();

        while current_height > 0 {
            trace!("Validating block at height {} ({})", current_height, current_hash);

            if current_height % 100 == 0 {
                debug!("Still validating; current height: {}", current_height);
            }

            if !self.block_hash_exists(&current_hash) {
                is_valid = false;
                error!("The header for block at height {} is missing!", current_height);
            }

            current_height -= 1;

            let previous_block = match self.get_block_from_block_number(current_height) {
                Ok(block) => block,
                Err(e) => {
                    error!("Couldn't find a block at height {}: {}!", current_height, e);
                    error!("The storage is invalid!");

                    return false;
                }
            };

            let previous_hash = previous_block.header.get_hash();

            if current_block.header.previous_block_hash != previous_hash {
                is_valid = false;
                error!(
                    "The parent hash of block at height {} doesn't match its child at {}!",
                    current_height + 1,
                    current_height,
                );
            }

            match self.get_child_block_hashes(&previous_hash) {
                Err(e) => {
                    is_valid = false;
                    error!("Can't find the children of block at height {}: {}!", previous_hash, e);
                }
                Ok(child_hashes) => {
                    if !child_hashes.contains(&current_hash) {
                        is_valid = false;
                        error!(
                            "The list of children hash of block at height {} don't contain the child at {}!",
                            current_height,
                            current_height + 1,
                        );
                    }
                }
            }

            if let Some(ref mut limit) = limit {
                *limit -= 1;
                if *limit == 0 {
                    info!("Specified block limit reached; the check is complete.");

                    return is_valid;
                }
            }

            current_block = previous_block;
            current_hash = previous_hash;
        }

        if is_valid {
            info!("The storage is valid!");
        } else {
            error!("The storage is invalid!");
        }

        is_valid
    }
}
