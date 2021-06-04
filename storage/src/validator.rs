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

use crate::{bytes_to_u32, Ledger, COL_META, KEY_BEST_BLOCK_NUMBER};
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_dpc::{errors::StorageError, Storage, TransactionScheme};

use tracing::*;

impl<T: TransactionScheme, P: LoadableMerkleParameters, S: Storage> Ledger<T, P, S> {
    pub fn validate(&self, mut limit: usize) -> Result<(), StorageError> {
        // Block height comes from the KEY_BEST_BLOCK_NUMBER.
        let mut current_height = self.get_current_block_height();

        if limit == 0 {
            info!("The limit of blocks to validate is 0; nothing to check.");

            return Ok(());
        }

        if current_height == 0 {
            info!("Only the genesis block is currently available; nothing to check.");

            return Ok(());
        }

        debug!("The block height is {}", current_height);

        match self.storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes()) {
            Err(_) | Ok(None) => error!("Can't obtain the best block number from storage!"),
            Ok(Some(n)) => {
                if bytes_to_u32(&n) != current_height {
                    error!("Current best block number doesn't match the block height!");
                }
            }
        }

        // Current block is found by COL_BLOCK_LOCATOR, as it should have been committed.
        let mut current_block = self.get_block_from_block_number(current_height);
        while let Err(e) = current_block {
            error!(
                "Couldn't find the latest block (height {}): {}! Trying a lower height next.",
                current_height, e
            );

            current_height -= 1;

            limit -= 1;
            if limit == 0 {
                info!("Specified block limit reached; the check is complete.");

                return Ok(());
            }

            current_block = self.get_block_from_block_number(current_height);
        }

        let mut current_block = current_block?;

        debug!("The true block height is {}", current_height);

        let mut current_hash = current_block.header.get_hash();

        while current_height > 0 {
            if current_height % 100 == 0 {
                debug!("Still validating; current height: {}", current_height);
            }

            if !self.block_hash_exists(&current_hash) {
                error!("The header for block at height {} is missing!", current_height);
            }

            current_height -= 1;

            let previous_block = self.get_block_from_block_number(current_height).map_err(|e| {
                error!("Couldn't find a block at height {}: {}!", current_height, e);
            })?;

            let previous_hash = previous_block.header.get_hash();

            if current_block.header.previous_block_hash != previous_hash {
                error!(
                    "The parent hash of block at height {} doesn't match its child at {}!",
                    current_height + 1,
                    current_height,
                );
            }

            match self.get_child_block_hashes(&previous_hash) {
                Err(e) => error!("Can't find the children of block at height {}: {}!", previous_hash, e),
                Ok(child_hashes) => {
                    if !child_hashes.contains(&current_hash) {
                        error!(
                            "The list of children hash of block at height {} don't contain the child at {}!",
                            current_height,
                            current_height + 1,
                        );
                    }
                }
            }

            limit -= 1;
            if limit == 0 {
                info!("Specified block limit reached; the check is complete.");

                return Ok(());
            }

            current_block = previous_block;
            current_hash = previous_hash;
        }

        info!("The storage was validated successfully!");

        Ok(())
    }
}
