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
use parking_lot::RwLock;
use std::{net::SocketAddr, sync::Arc};

/// The number of block locators.
const NUM_LOCATORS: usize = 100; // 100 locators
/// The interval between block locators.
const LOCATOR_INTERVAL: u32 = 1; // 1 block intervals
/// The interval between block checkpoints.
const CHECKPOINT_INTERVAL: u32 = 10_000; // 10,000 block intervals

/// The assumed structure (in order of block height) is: `\[ checkpoints || locators || height \]`
#[derive(Clone, Debug)]
pub struct Sync<N: Network> {
    /// The map of peer IPs to their last known block height.
    heights: Arc<RwLock<IndexMap<SocketAddr, u32>>>,
    /// The map of peer IPs to their block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, IndexMap<u32, N::BlockHash>>>>,
    /// The map of peer IPs to their block checkpoints.
    checkpoints: Arc<RwLock<IndexMap<SocketAddr, IndexMap<u32, N::BlockHash>>>>,
}

impl<N: Network> Default for Sync<N> {
    /// Initializes a new instance of the sync manager.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Sync<N> {
    /// Initializes a new instance of the sync manager.
    pub fn new() -> Self {
        Self { heights: Default::default(), locators: Default::default(), checkpoints: Default::default() }
    }

    /// Updates the last known block height and block checkpoints for the given peer IP.
    /// This function ensures the given locators and checkpoints are consistent with the previous ones.
    pub fn update_peer(
        &self,
        peer_ip: SocketAddr,
        height: u32,
        locators: IndexMap<u32, N::BlockHash>,
        checkpoints: IndexMap<u32, N::BlockHash>,
    ) -> Result<()> {
        // Ensure the block locators are well-formed.
        let last_locator_height = Self::check_block_locators(height, &locators)?;
        // Ensure the block checkpoints are well-formed.
        let last_checkpoint_height = Self::check_block_checkpoints(height, &checkpoints)?;
        // Ensure the last locator height is at or above the last checkpoint height.
        if last_locator_height < last_checkpoint_height {
            bail!("Locators ({last_locator_height}) cannot be below checkpoints ({last_checkpoint_height})");
        }

        // Acquire the write lock on the heights map.
        let mut heights_write = self.heights.write();
        // Acquire the write lock on the locators map.
        let mut locators_write = self.locators.write();
        // Acquire the write lock on the checkpoints map.
        let mut checkpoints_write = self.checkpoints.write();

        // Retrieve the height entry for the given peer IP.
        let height_entry = heights_write.entry(peer_ip).or_default();
        // Retrieve the locators entry for the given peer IP.
        let locators_entry = locators_write.entry(peer_ip).or_default();
        // Retrieve the checkpoints entry for the given peer IP.
        let checkpoints_entry = checkpoints_write.entry(peer_ip).or_default();

        // For the overlapping block locators, ensure their block hashes match.
        for (height, hash) in locators.iter() {
            if let Some(locator_hash) = locators_entry.get(height) {
                if locator_hash != hash {
                    bail!("Block locator hash mismatch for height {height}");
                }
            }
        }
        // For the overlapping block checkpoints, ensure their block hashes match.
        for (height, hash) in checkpoints.iter() {
            if let Some(checkpoint_hash) = checkpoints_entry.get(height) {
                if checkpoint_hash != hash {
                    bail!("Block checkpoint hash mismatch for height {height}");
                }
            }
        }

        // Update the height entry for the given peer IP.
        *height_entry = height;
        // Update the locators entry for the given peer IP.
        *locators_entry = locators;
        // Update the checkpoints entry for the given peer IP.
        *checkpoints_entry = checkpoints;

        Ok(())
    }

    /// Checks the block locators, returning the last block height from the locators.
    ///
    /// This function checks the following:
    /// 1. The block locators are not empty.
    /// 2. The block locators are at the correct interval.
    /// 3. The block locators are at the correct height.
    /// 4. The block locators are in the correct order.
    /// 5. The block locators are not too many.
    /// 6. The given height is the last block locator.
    fn check_block_locators(height: u32, locators: &IndexMap<u32, N::BlockHash>) -> Result<u32> {
        // Ensure the number of locators is at least 1.
        if locators.is_empty() {
            bail!("Block locators must contain at least 1 entry")
        }
        // Ensure the number of locators is at most NUM_LOCATORS.
        // This redundant check ensures we early exit if the number of locators is too large.
        if locators.len() > NUM_LOCATORS {
            bail!("Block locators must be at most {NUM_LOCATORS}")
        }

        // Ensure the given locators increment in height, and at the correct interval.
        let mut last_height = 0;
        for current_height in locators.keys() {
            if *current_height <= last_height {
                bail!("Block locators must increment in height")
            }
            if *current_height - last_height != LOCATOR_INTERVAL {
                bail!("Block locators must increment by {LOCATOR_INTERVAL}")
            }
            last_height = *current_height;
        }

        // Ensure the given height is the last locator height.
        if height != last_height {
            bail!("Block height must be the last locator height")
        }

        // If the last height is below NUM_LOCATORS, ensure the number of locators matches the last height.
        if last_height < NUM_LOCATORS as u32 && locators.len() as u32 != last_height {
            bail!("As the last height is below {NUM_LOCATORS}, the number of block locators must match the height")
        }
        // Otherwise ensure the number of locators matches NUM_LOCATORS.
        else if last_height >= NUM_LOCATORS as u32 && locators.len() != NUM_LOCATORS {
            bail!("Number of block locators must match {NUM_LOCATORS}")
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
    /// 5. The given height is at or above the last checkpoint height.
    fn check_block_checkpoints(height: u32, checkpoints: &IndexMap<u32, <N as Network>::BlockHash>) -> Result<u32> {
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
            if *current_height - last_height != CHECKPOINT_INTERVAL {
                bail!("Block checkpoints must increment by {CHECKPOINT_INTERVAL}")
            }
            last_height = *current_height;
        }

        // Ensure the given height is at or above the last checkpoint height.
        if height < last_height {
            bail!("Block height must be at or above the last checkpoint height")
        }

        Ok(last_height)
    }
}
