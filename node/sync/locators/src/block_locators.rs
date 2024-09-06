// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use snarkvm::prelude::{error, has_duplicates, FromBytes, IoResult, Network, Read, ToBytes, Write};

use anyhow::{bail, ensure, Result};
use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};
use std::collections::{btree_map::IntoIter, BTreeMap};

/// The number of recent blocks (near tip).
pub const NUM_RECENT_BLOCKS: usize = 100; // 100 blocks
/// The interval between recent blocks.
const RECENT_INTERVAL: u32 = 1; // 1 block intervals
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
    /// Initializes a new instance of the block locators, checking the validity of the block locators.
    pub fn new(recents: IndexMap<u32, N::BlockHash>, checkpoints: IndexMap<u32, N::BlockHash>) -> Result<Self> {
        // Construct the block locators.
        let locators = Self { recents, checkpoints };
        // Ensure the block locators are well-formed.
        locators.ensure_is_valid()?;
        // Return the block locators.
        Ok(locators)
    }

    /// Initializes a new instance of the block locators, without checking the validity of the block locators.
    pub fn new_unchecked(recents: IndexMap<u32, N::BlockHash>, checkpoints: IndexMap<u32, N::BlockHash>) -> Self {
        Self { recents, checkpoints }
    }

    /// Initializes a new genesis instance of the block locators.
    pub fn new_genesis(genesis_hash: N::BlockHash) -> Self {
        Self { recents: indexmap![0 => genesis_hash], checkpoints: indexmap![0 => genesis_hash] }
    }
}

impl<N: Network> IntoIterator for BlockLocators<N> {
    type IntoIter = IntoIter<u32, N::BlockHash>;
    type Item = (u32, N::BlockHash);

    // TODO (howardwu): Consider using `BTreeMap::from_par_iter` if it is more performant.
    //  Check by sorting 300-1000 items and comparing the performance.
    //  (https://docs.rs/indexmap/latest/indexmap/map/struct.IndexMap.html#method.from_par_iter)
    fn into_iter(self) -> Self::IntoIter {
        BTreeMap::from_iter(self.checkpoints.into_iter().chain(self.recents)).into_iter()
    }
}

impl<N: Network> BlockLocators<N> {
    /// Returns the latest locator height.
    pub fn latest_locator_height(&self) -> u32 {
        self.recents.keys().last().copied().unwrap_or_default()
    }

    /// Returns the block hash for the given block height, if it exists.
    pub fn get_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.recents.get(&height).copied().or_else(|| self.checkpoints.get(&height).copied())
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
        recents: &IndexMap<u32, N::BlockHash>,
        checkpoints: &IndexMap<u32, N::BlockHash>,
    ) -> Result<()> {
        // Ensure the recent blocks are well-formed.
        let last_recent_height = Self::check_recent_blocks(recents)?;
        // Ensure the block checkpoints are well-formed.
        let last_checkpoint_height = Self::check_block_checkpoints(checkpoints)?;

        // Ensure the `last_recent_height` is at or above `last_checkpoint_height - NUM_RECENT_BLOCKS`.
        let threshold = last_checkpoint_height.saturating_sub(NUM_RECENT_BLOCKS as u32);
        if last_recent_height < threshold {
            bail!("Recent height ({last_recent_height}) cannot be below checkpoint threshold ({threshold})")
        }

        // If the `last_recent_height` is below NUM_RECENT_BLOCKS, ensure the genesis hash matches in both maps.
        if last_recent_height < NUM_RECENT_BLOCKS as u32
            && recents.get(&0).copied().unwrap_or_default() != checkpoints.get(&0).copied().unwrap_or_default()
        {
            bail!("Recent genesis hash and checkpoint genesis hash mismatch at height {last_recent_height}")
        }

        // If the `last_recent_height` overlaps with a checkpoint, ensure the block hashes match.
        if let Some(last_checkpoint_hash) = checkpoints.get(&last_recent_height) {
            if let Some(last_recent_hash) = recents.get(&last_recent_height) {
                if last_checkpoint_hash != last_recent_hash {
                    bail!("Recent block hash and checkpoint hash mismatch at height {last_recent_height}")
                }
            }
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
        // Ensure the number of recent blocks is at most NUM_RECENT_BLOCKS.
        // This redundant check ensures we early exit if the number of recent blocks is too large.
        if recents.len() > NUM_RECENT_BLOCKS {
            bail!("There can be at most {NUM_RECENT_BLOCKS} blocks in the map")
        }

        // Ensure the given recent blocks increment in height, and at the correct interval.
        let mut last_height = 0;
        for (i, current_height) in recents.keys().enumerate() {
            if i == 0 && recents.len() < NUM_RECENT_BLOCKS && *current_height != last_height {
                bail!("Ledgers under {NUM_RECENT_BLOCKS} blocks must have the first recent block at height 0")
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
        if last_height < NUM_RECENT_BLOCKS as u32 && recents.len().saturating_sub(1) as u32 != last_height {
            bail!("As the last height is below {NUM_RECENT_BLOCKS}, the number of recent blocks must match the height")
        }
        // Otherwise, ensure the number of recent blocks matches NUM_RECENT_BLOCKS.
        if last_height >= NUM_RECENT_BLOCKS as u32 && recents.len() != NUM_RECENT_BLOCKS {
            bail!("Number of recent blocks must match {NUM_RECENT_BLOCKS}")
        }

        // Ensure the block hashes are unique.
        if has_duplicates(recents.values()) {
            bail!("Recent block hashes must be unique")
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
    fn check_block_checkpoints(checkpoints: &IndexMap<u32, N::BlockHash>) -> Result<u32> {
        // Ensure the block checkpoints are not empty.
        ensure!(!checkpoints.is_empty(), "There must be at least 1 block checkpoint");

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

        // Ensure the block hashes are unique.
        if has_duplicates(checkpoints.values()) {
            bail!("Block checkpoints must be unique")
        }

        Ok(last_height)
    }
}

impl<N: Network> FromBytes for BlockLocators<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the number of recent block hashes.
        let num_recents = u32::read_le(&mut reader)?;
        // Read the recent block hashes.
        let mut recents = IndexMap::new();
        for _ in 0..num_recents {
            let height = u32::read_le(&mut reader)?;
            let hash = N::BlockHash::read_le(&mut reader)?;
            recents.insert(height, hash);
        }

        // Read the number of checkpoints.
        let num_checkpoints = u32::read_le(&mut reader)?;
        // Read the checkpoints.
        let mut checkpoints = IndexMap::new();
        for _ in 0..num_checkpoints {
            let height = u32::read_le(&mut reader)?;
            let hash = N::BlockHash::read_le(&mut reader)?;
            checkpoints.insert(height, hash);
        }

        Self::new(recents, checkpoints).map_err(error)
    }
}

impl<N: Network> ToBytes for BlockLocators<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the number of recent block hashes.
        u32::try_from(self.recents.len()).map_err(error)?.write_le(&mut writer)?;
        // Write the recent block hashes.
        for (height, hash) in &self.recents {
            height.write_le(&mut writer)?;
            hash.write_le(&mut writer)?;
        }

        // Write the number of checkpoints.
        u32::try_from(self.checkpoints.len()).map_err(error)?.write_le(&mut writer)?;
        // Write the checkpoints.
        for (height, hash) in &self.checkpoints {
            height.write_le(&mut writer)?;
            hash.write_le(&mut writer)?;
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "test"))]
pub mod test_helpers {
    use super::*;
    use snarkvm::prelude::Field;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    /// Simulates a block locator at the given height.
    pub fn sample_block_locators(height: u32) -> BlockLocators<CurrentNetwork> {
        // Create the recent locators.
        let mut recents = IndexMap::new();
        let recents_range = match height < NUM_RECENT_BLOCKS as u32 {
            true => 0..=height,
            false => (height - NUM_RECENT_BLOCKS as u32 + 1)..=height,
        };
        for i in recents_range {
            recents.insert(i, (Field::<CurrentNetwork>::from_u32(i)).into());
        }

        // Create the checkpoint locators.
        let mut checkpoints = IndexMap::new();
        for i in (0..=height).step_by(CHECKPOINT_INTERVAL as usize) {
            checkpoints.insert(i, (Field::<CurrentNetwork>::from_u32(i)).into());
        }

        // Construct the block locators.
        BlockLocators::new(recents, checkpoints).unwrap()
    }

    /// Simulates a block locator at the given height, with a fork within NUM_RECENTS of the given height.
    pub fn sample_block_locators_with_fork(height: u32, fork_height: u32) -> BlockLocators<CurrentNetwork> {
        assert!(fork_height <= height, "Fork height must be less than or equal to the given height");
        assert!(height - fork_height < NUM_RECENT_BLOCKS as u32, "Fork must be within NUM_RECENTS of the given height");

        // Create the recent locators.
        let mut recents = IndexMap::new();
        let recents_range = match height < NUM_RECENT_BLOCKS as u32 {
            true => 0..=height,
            false => (height - NUM_RECENT_BLOCKS as u32 + 1)..=height,
        };
        for i in recents_range {
            if i >= fork_height {
                recents.insert(i, (-Field::<CurrentNetwork>::from_u32(i)).into());
            } else {
                recents.insert(i, (Field::<CurrentNetwork>::from_u32(i)).into());
            }
        }

        // Create the checkpoint locators.
        let mut checkpoints = IndexMap::new();
        for i in (0..=height).step_by(CHECKPOINT_INTERVAL as usize) {
            checkpoints.insert(i, (Field::<CurrentNetwork>::from_u32(i)).into());
        }

        // Construct the block locators.
        BlockLocators::new(recents, checkpoints).unwrap()
    }

    /// A test to ensure that the sample block locators are valid.
    #[test]
    fn test_sample_block_locators() {
        for expected_height in 0..=100_001u32 {
            println!("Testing height - {expected_height}");

            let expected_num_checkpoints = (expected_height / CHECKPOINT_INTERVAL) + 1;
            let expected_num_recents = match expected_height < NUM_RECENT_BLOCKS as u32 {
                true => expected_height + 1,
                false => NUM_RECENT_BLOCKS as u32,
            };

            let block_locators = sample_block_locators(expected_height);
            assert_eq!(block_locators.checkpoints.len(), expected_num_checkpoints as usize);
            assert_eq!(block_locators.recents.len(), expected_num_recents as usize);
            assert_eq!(block_locators.latest_locator_height(), expected_height);
            assert!(block_locators.is_valid());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::prelude::Field;

    use core::ops::Range;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    /// Simulates block locators for a ledger within the given `heights` range.
    fn check_is_valid(checkpoints: IndexMap<u32, <CurrentNetwork as Network>::BlockHash>, heights: Range<u32>) {
        for height in heights {
            let mut recents = IndexMap::new();
            for i in 0..NUM_RECENT_BLOCKS as u32 {
                recents.insert(height + i, (Field::<CurrentNetwork>::from_u32(height + i)).into());

                let block_locators =
                    BlockLocators::<CurrentNetwork>::new_unchecked(recents.clone(), checkpoints.clone());
                if height == 0 && recents.len() < NUM_RECENT_BLOCKS {
                    // For the first NUM_RECENTS blocks, ensure NUM_RECENTS - 1 or less is valid.
                    block_locators.ensure_is_valid().unwrap();
                } else if recents.len() < NUM_RECENT_BLOCKS {
                    // After the first NUM_RECENTS blocks from genesis, ensure NUM_RECENTS - 1 or less is not valid.
                    block_locators.ensure_is_valid().unwrap_err();
                } else {
                    // After the first NUM_RECENTS blocks from genesis, ensure NUM_RECENTS is valid.
                    block_locators.ensure_is_valid().unwrap();
                }
            }
            // Ensure NUM_RECENTS + 1 is not valid.
            recents.insert(
                height + NUM_RECENT_BLOCKS as u32,
                (Field::<CurrentNetwork>::from_u32(height + NUM_RECENT_BLOCKS as u32)).into(),
            );
            let block_locators = BlockLocators::<CurrentNetwork>::new_unchecked(recents.clone(), checkpoints.clone());
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
            for i in 0..NUM_RECENT_BLOCKS as u32 {
                recents.insert(height + i, (Field::<CurrentNetwork>::from_u32(height + i)).into());

                let block_locators =
                    BlockLocators::<CurrentNetwork>::new_unchecked(recents.clone(), checkpoints.clone());
                block_locators.ensure_is_consistent_with(&block_locators).unwrap();

                // Only test consistency when the block locators are valid to begin with.
                let is_first_num_recents_blocks = height == 0 && recents.len() < NUM_RECENT_BLOCKS;
                let is_num_recents_blocks = recents.len() == NUM_RECENT_BLOCKS;
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
        let zero: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(0)).into();
        let checkpoint_1: <CurrentNetwork as Network>::BlockHash =
            (Field::<CurrentNetwork>::from_u32(CHECKPOINT_INTERVAL)).into();

        // Ensure the block locators are valid.
        for height in 0..10 {
            let block_locators = test_helpers::sample_block_locators(height);
            block_locators.ensure_is_valid().unwrap();
        }

        // Ensure the first NUM_RECENT blocks are valid.
        let checkpoints = IndexMap::from([(0, zero)]);
        let mut recents = IndexMap::new();
        for i in 0..NUM_RECENT_BLOCKS {
            recents.insert(i as u32, (Field::<CurrentNetwork>::from_u32(i as u32)).into());
            let block_locators = BlockLocators::<CurrentNetwork>::new(recents.clone(), checkpoints.clone()).unwrap();
            block_locators.ensure_is_valid().unwrap();
        }
        // Ensure NUM_RECENTS + 1 is not valid.
        recents.insert(NUM_RECENT_BLOCKS as u32, (Field::<CurrentNetwork>::from_u32(NUM_RECENT_BLOCKS as u32)).into());
        let block_locators = BlockLocators::<CurrentNetwork>::new_unchecked(recents.clone(), checkpoints);
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure block locators before the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, zero)]);
        check_is_valid(checkpoints, 0..(CHECKPOINT_INTERVAL - NUM_RECENT_BLOCKS as u32));

        // Ensure the block locators after the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, zero), (CHECKPOINT_INTERVAL, checkpoint_1)]);
        check_is_valid(
            checkpoints,
            (CHECKPOINT_INTERVAL - NUM_RECENT_BLOCKS as u32)..(CHECKPOINT_INTERVAL * 2 - NUM_RECENT_BLOCKS as u32),
        );
    }

    #[test]
    fn test_ensure_is_valid_fails() {
        let zero: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(0)).into();
        let one: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(1)).into();

        // Ensure an empty block locators is not valid.
        let block_locators = BlockLocators::<CurrentNetwork>::new_unchecked(Default::default(), Default::default());
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure internally-mismatching genesis block locators is valid.
        let block_locators =
            BlockLocators::<CurrentNetwork>::new_unchecked(IndexMap::from([(0, zero)]), IndexMap::from([(0, one)]));
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure internally-mismatching genesis block locators is valid.
        let block_locators =
            BlockLocators::<CurrentNetwork>::new_unchecked(IndexMap::from([(0, one)]), IndexMap::from([(0, zero)]));
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure internally-mismatching block locators with two recent blocks is valid.
        let block_locators = BlockLocators::<CurrentNetwork>::new_unchecked(
            IndexMap::from([(0, one), (1, zero)]),
            IndexMap::from([(0, zero)]),
        );
        block_locators.ensure_is_valid().unwrap_err();

        // Ensure duplicate recent block hashes are not valid.
        let block_locators = BlockLocators::<CurrentNetwork>::new_unchecked(
            IndexMap::from([(0, zero), (1, zero)]),
            IndexMap::from([(0, zero)]),
        );
        block_locators.ensure_is_valid().unwrap_err();
    }

    #[test]
    fn test_ensure_is_consistent_with() {
        let zero: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(0)).into();
        let one: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(1)).into();

        let genesis_locators =
            BlockLocators::<CurrentNetwork>::new_unchecked(IndexMap::from([(0, zero)]), IndexMap::from([(0, zero)]));
        let second_locators = BlockLocators::<CurrentNetwork>::new_unchecked(
            IndexMap::from([(0, zero), (1, one)]),
            IndexMap::from([(0, zero)]),
        );

        // Ensure genesis block locators is consistent with genesis block locators.
        genesis_locators.ensure_is_consistent_with(&genesis_locators).unwrap();

        // Ensure genesis block locators is consistent with block locators with two recent blocks.
        genesis_locators.ensure_is_consistent_with(&second_locators).unwrap();
        second_locators.ensure_is_consistent_with(&genesis_locators).unwrap();

        // Ensure the block locators before the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, Default::default())]);
        check_is_consistent(
            checkpoints,
            0..(CHECKPOINT_INTERVAL - NUM_RECENT_BLOCKS as u32),
            genesis_locators.clone(),
            second_locators.clone(),
        );

        // Ensure the block locators after the second checkpoint are valid.
        let checkpoints = IndexMap::from([(0, Default::default()), (CHECKPOINT_INTERVAL, Default::default())]);
        check_is_consistent(
            checkpoints,
            (CHECKPOINT_INTERVAL - NUM_RECENT_BLOCKS as u32)..(CHECKPOINT_INTERVAL * 2 - NUM_RECENT_BLOCKS as u32),
            genesis_locators,
            second_locators,
        );
    }

    #[test]
    fn test_ensure_is_consistent_with_fails() {
        let zero: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(0)).into();
        let one: <CurrentNetwork as Network>::BlockHash = (Field::<CurrentNetwork>::from_u32(1)).into();

        let genesis_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, zero)]), IndexMap::from([(0, zero)])).unwrap();
        let second_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, zero), (1, one)]), IndexMap::from([(0, zero)]))
                .unwrap();

        let wrong_genesis_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, one)]), IndexMap::from([(0, one)])).unwrap();
        let wrong_second_locators =
            BlockLocators::<CurrentNetwork>::new(IndexMap::from([(0, one), (1, zero)]), IndexMap::from([(0, one)]))
                .unwrap();

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
