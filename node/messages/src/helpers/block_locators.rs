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

use snarkvm::prelude::Network;

use anyhow::{bail, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The number of recent blocks (near tip).
pub const NUM_RECENTS: usize = 100; // 100 blocks
/// The interval between recent blocks.
pub const RECENT_INTERVAL: u32 = 1; // 1 block intervals
/// The interval between block checkpoints.
pub const CHECKPOINT_INTERVAL: u32 = 10_000; // 10,000 block intervals

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockLocators<N: Network> {
    /// The map of recent blocks.
    pub recents: IndexMap<u32, N::BlockHash>,
    /// The map of block checkpoints.
    pub checkpoints: IndexMap<u32, N::BlockHash>,
}

impl<N: Network> BlockLocators<N> {
    /// Initializes a new instance of the block locators.
    pub fn new(recents: IndexMap<u32, N::BlockHash>, checkpoints: IndexMap<u32, N::BlockHash>) -> Self {
        Self { recents, checkpoints }
    }

    /// Returns the latest height.
    pub fn height(&self) -> u32 {
        self.recents.keys().last().copied().unwrap_or_default()
    }

    /// Returns `true` if the block locators are well-formed.
    pub fn is_valid(&self) -> bool {
        // Ensure the block locators are well-formed.
        if let Err(error) = self.ensure_is_valid() {
            warn!("Block locators are invalid: {error}");
            return false;
        }
        true
    }

    /// Returns `true` if the given block locators are consistent with this one.
    /// This function assumes the given block locators are well-formed.
    pub fn is_consistent_with(&self, other: &Self) -> bool {
        // Ensure the block locators are consistent with the previous ones.
        if let Err(error) = self.ensure_is_consistent_with(other) {
            warn!("Inconsistent block locators: {error}");
            return false;
        }
        true
    }

    /// Checks that this block locators are well-formed.
    pub fn ensure_is_valid(&self) -> Result<()> {
        // Ensure the block locators are well-formed.
        Self::check_block_locators(&self.recents, &self.checkpoints)
    }

    /// Returns `true` if the given block locators are consistent with this one.
    /// This function assumes the given block locators are well-formed.
    pub fn ensure_is_consistent_with(&self, other: &Self) -> Result<()> {
        Self::check_consistent_block_locators(self, other)
    }
}

impl<N: Network> BlockLocators<N> {
    /// Checks the old and new block locators share a consistent view of block history.
    /// This function assumes the given block locators are well-formed.
    pub fn check_consistent_block_locators(
        old_locators: &BlockLocators<N>,
        new_locators: &BlockLocators<N>,
    ) -> Result<()> {
        // For the overlapping recent blocks, ensure their block hashes match.
        for (height, hash) in new_locators.recents.iter() {
            if let Some(recent_hash) = old_locators.recents.get(height) {
                if recent_hash != hash {
                    bail!("Recent block hash mismatch at height {height}")
                }
            }
        }
        // For the overlapping block checkpoints, ensure their block hashes match.
        for (height, hash) in new_locators.checkpoints.iter() {
            if let Some(checkpoint_hash) = old_locators.checkpoints.get(height) {
                if checkpoint_hash != hash {
                    bail!("Block checkpoint hash mismatch for height {height}")
                }
            }
        }
        Ok(())
    }

    /// Checks that the block locators are well-formed.
    pub fn check_block_locators(
        recents: &IndexMap<u32, <N as Network>::BlockHash>,
        checkpoints: &IndexMap<u32, <N as Network>::BlockHash>,
    ) -> Result<()> {
        // Ensure the recent blocks are well-formed.
        let last_recent_height = Self::check_recent_blocks(recents)?;
        // Ensure the block checkpoints are well-formed.
        let last_checkpoint_height = Self::check_block_checkpoints(checkpoints)?;
        // Ensure the `last_recent_height` is at or above `last_checkpoint_height - NUM_RECENTS`.
        let threshold = last_checkpoint_height.saturating_sub(NUM_RECENTS as u32);
        if last_recent_height < threshold {
            bail!("Recent height ({last_recent_height}) cannot be below checkpoint threshold ({threshold})")
        }
        Ok(())
    }

    /// Checks the recent blocks, returning the last block height from the map.
    ///
    /// This function checks the following:
    /// 1. The map is not empty.
    /// 2. The map is at the correct interval.
    /// 3. The map is at the correct height.
    /// 4. The map is in the correct order.
    /// 5. The map does not contain too many entries.
    fn check_recent_blocks(recents: &IndexMap<u32, N::BlockHash>) -> Result<u32> {
        // Ensure the number of recent blocks is at least 1.
        if recents.is_empty() {
            bail!("There must be at least 1 recent block")
        }
        // Ensure the number of recent blocks is at most NUM_RECENTS.
        // This redundant check ensures we early exit if the number of recent blocks is too large.
        if recents.len() > NUM_RECENTS {
            bail!("There can be at most {NUM_RECENTS} blocks in the map")
        }

        // Ensure the given recent blocks increment in height, and at the correct interval.
        let mut last_height = 0;
        for (i, current_height) in recents.keys().enumerate() {
            if i == 0 && recents.len() < NUM_RECENTS && *current_height != last_height {
                bail!("Ledgers under {NUM_RECENTS} blocks must have the first recent block at height 0")
            }
            if i > 0 && *current_height <= last_height {
                bail!("Recent blocks must increment in height")
            }
            if i > 0 && *current_height - last_height != RECENT_INTERVAL {
                bail!("Recent blocks must increment by {RECENT_INTERVAL}")
            }
            last_height = *current_height;
        }

        // If the last height is below NUM_RECENTS, ensure the number of recent blocks matches the last height.
        if last_height < NUM_RECENTS as u32 && recents.len().saturating_sub(1) as u32 != last_height {
            bail!("As the last height is below {NUM_RECENTS}, the number of recent blocks must match the height")
        }
        // Otherwise, ensure the number of recent blocks matches NUM_RECENTS.
        if last_height >= NUM_RECENTS as u32 && recents.len() != NUM_RECENTS {
            bail!("Number of recent blocks must match {NUM_RECENTS}")
        }

        Ok(last_height)
    }

    /// Checks the block checkpoints, returning the last block height from the checkpoints.
    ///
    /// This function checks the following:
    /// 1. The block checkpoints are not empty.
    /// 2. The block checkpoints are at the correct interval.
    /// 3. The block checkpoints are at the correct height.
    /// 4. The block checkpoints are in the correct order.
    fn check_block_checkpoints(checkpoints: &IndexMap<u32, <N as Network>::BlockHash>) -> Result<u32> {
        // Ensure the block checkpoints are not empty.
        assert!(!checkpoints.is_empty());

        // Ensure the given checkpoints increment in height, and at the correct interval.
        let mut last_height = 0;
        for (i, current_height) in checkpoints.keys().enumerate() {
            if i == 0 && *current_height != 0 {
                bail!("First block checkpoint must be at height 0")
            }
            if i > 0 && *current_height <= last_height {
                bail!("Block checkpoints must increment in height")
            }
            if i > 0 && *current_height - last_height != CHECKPOINT_INTERVAL {
                bail!("Block checkpoints must increment by {CHECKPOINT_INTERVAL}")
            }
            last_height = *current_height;
        }

        Ok(last_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::prelude::Field;

    use core::ops::Range;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    /// Simulates block locators for a ledger within the given `heights` range.
    fn check_is_valid(checkpoints: IndexMap<u32, <CurrentNetwork as Network>::BlockHash>, heights: Range<u32>) {
        for height in heights {
            let mut recents = IndexMap::new();
            for i in 0..NUM_RECENTS as u32 {
                recents.insert(height + i, Default::default());

                let block_locators = BlockLocators::<CurrentNetwork>::new(recents.clone(), checkpoints.clone());
                if height == 0 && recents.len() < NUM_RECENTS {
                    // For the first NUM_RECENTS blocks, ensure NUM_RECENTS - 1 or less is valid.
                    block_locators.ensure_is_valid().unwrap();
                } else if recents.len() < NUM_RECENTS {
                    // After the first NUM_RECENTS blocks from genesis, ensure NUM_RECENTS - 1 or less is not valid.
                    block_locators.ensure_is_valid().unwrap_err();
                } else {
                    // After the first NUM_RECENTS blocks from genesis, ensure NUM_RECENTS is valid.
                    block_locators.ensure_is_valid().unwrap();
                }
            }
            // Ensure NUM_RECENTS + 1 is not valid.
            recents.insert(height + NUM_RECENTS as u32, Default::default());
            let block_locators = BlockLocators::<CurrentNetwork>::new(recents.clone(), checkpoints.clone());
            block_locators.ensure_is_valid().unwrap_err();
        }
    }

    /// Simulates block locators for a ledger within the given `heights` range.
    fn check_is_consistent(
        checkpoints: IndexMap<u32, <CurrentNetwork as Network>::BlockHash>,
        heights: Range<u32>,
        genesis_locators: BlockLocators<CurrentNetwork>,
        second_locators: BlockLocators<CurrentNetwork>,
    ) {
        for height in heights {
            let mut recents = IndexMap::new();
            for i in 0..NUM_RECENTS as u32 {
                // We make the block hash a unique number to ensure consistency is tested.
                let dummy_hash: <CurrentNetwork as Network>::BlockHash =
                    (Field::<CurrentNetwork>::from_u32(height + i)).into();
                recents.insert(height + i, dummy_hash);

                let block_locators = BlockLocators::<CurrentNetwork>::new(recents.clone(), checkpoints.clone());
                block_locators.ensure_is_consistent_with(&block_locators).unwrap();

                // Only test consistency when the block locators are valid to begin with.
                let is_first_num_recents_blocks = height == 0 && recents.len() < NUM_RECENTS;
                let is_num_recents_blocks = recents.len() == NUM_RECENTS;
                if is_first_num_recents_blocks || is_num_recents_blocks {
                    // Ensure the block locators are consistent with the genesis block locators.
                    genesis_locators.ensure_is_consistent_with(&block_locators).unwrap();
                    block_locators.ensure_is_consistent_with(&genesis_locators).unwrap();

                    // Ensure the block locators are consistent with the block locators with two recent blocks.
                    second_locators.ensure_is_consistent_with(&block_locators).unwrap();
                    block_locators.ensure_is_consistent_with(&second_locators).unwrap();
                }
            }
        }
    }

    #[test]
    fn test_ensure_is_valid() {
        // Ensure an empty block locators is not valid.
        let block_locators = BlockLocators::<CurrentNetwork>::new(Default::default(), Default::default());
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure genesis block locators is valid.
        let block_locators = BlockLocators::<CurrentNetwork>::new(
            IndexMap::from([(0, Default::default())]),
            IndexMap::from([(0, Default::default())]),
        );
        block_locators.ensure_is_valid().unwrap();

        // Ensure block locators with two recent blocks is valid.
        let block_locators = BlockLocators::<CurrentNetwork>::new(
            IndexMap::from([(0, Default::default()), (1, Default::default())]),
            IndexMap::from([(0, Default::default())]),
        );
        block_locators.ensure_is_valid().unwrap();

        // Ensure the first NUM_RECENT blocks are valid.
        let checkpoints = IndexMap::from([(0, Default::default())]);
        let mut recents = IndexMap::new();
        for i in 0..NUM_RECENTS {
            recents.insert(i as u32, Default::default());
            let block_locators = BlockLocators::<CurrentNetwork>::new(recents.clone(), checkpoints.clone());
            block_locators.ensure_is_valid().unwrap();
        }
        // Ensure NUM_RECENTS + 1 is not valid.
        recents.insert(NUM_RECENTS as u32, Default::default());
        let block_locators = BlockLocators::<CurrentNetwork>::new(recents.clone(), checkpoints);
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure block locators before the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, Default::default())]);
        check_is_valid(checkpoints, 0..(CHECKPOINT_INTERVAL - NUM_RECENTS as u32));

        // Ensure the block locators after the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, Default::default()), (CHECKPOINT_INTERVAL, Default::default())]);
        check_is_valid(
            checkpoints,
            (CHECKPOINT_INTERVAL - NUM_RECENTS as u32)..(CHECKPOINT_INTERVAL * 2 - NUM_RECENTS as u32),
        );
    }

    #[test]
    fn test_ensure_is_consistent_with() {
        let zero: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(0)).into();
        let one: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(1)).into();

        let genesis_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, zero)]), IndexMap::from([(0, zero)]));
        let second_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, zero), (1, one)]), IndexMap::from([(0, zero)]));

        // Ensure genesis block locators is consistent with genesis block locators.
        genesis_locators.ensure_is_consistent_with(&genesis_locators).unwrap();

        // Ensure genesis block locators is consistent with block locators with two recent blocks.
        genesis_locators.ensure_is_consistent_with(&second_locators).unwrap();
        second_locators.ensure_is_consistent_with(&genesis_locators).unwrap();

        // Ensure the block locators before the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, Default::default())]);
        check_is_consistent(
            checkpoints,
            0..(CHECKPOINT_INTERVAL - NUM_RECENTS as u32),
            genesis_locators.clone(),
            second_locators.clone(),
        );

        // Ensure the block locators after the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, Default::default()), (CHECKPOINT_INTERVAL, Default::default())]);
        check_is_consistent(
            checkpoints,
            (CHECKPOINT_INTERVAL - NUM_RECENTS as u32)..(CHECKPOINT_INTERVAL * 2 - NUM_RECENTS as u32),
            genesis_locators,
            second_locators,
        );
    }

    #[test]
    fn test_ensure_is_consistent_with_fails() {
        let zero: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(0)).into();
        let one: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(1)).into();

        let genesis_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, zero)]), IndexMap::from([(0, zero)]));
        let second_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, zero), (1, one)]), IndexMap::from([(0, zero)]));

        let wrong_genesis_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, one)]), IndexMap::from([(0, one)]));
        let wrong_second_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, one), (1, zero)]), IndexMap::from([(0, one)]));

        genesis_locators.ensure_is_consistent_with(&wrong_genesis_locators).unwrap_err();
        wrong_genesis_locators.ensure_is_consistent_with(&genesis_locators).unwrap_err();

        genesis_locators.ensure_is_consistent_with(&wrong_second_locators).unwrap_err();
        wrong_second_locators.ensure_is_consistent_with(&genesis_locators).unwrap_err();

        second_locators.ensure_is_consistent_with(&wrong_genesis_locators).unwrap_err();
        wrong_genesis_locators.ensure_is_consistent_with(&second_locators).unwrap_err();

        second_locators.ensure_is_consistent_with(&wrong_second_locators).unwrap_err();
        wrong_second_locators.ensure_is_consistent_with(&second_locators).unwrap_err();
    }
}
