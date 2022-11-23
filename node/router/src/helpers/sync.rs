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
use itertools::Itertools;
use parking_lot::RwLock;
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub struct Sync<N: Network> {
    /// The map of peer IPs to their block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, BlockLocators<N>>>>,
}

impl<N: Network> Default for Sync<N> {
    /// Initializes a new instance of the sync module.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Sync<N> {
    /// Initializes a new instance of the sync module.
    pub fn new() -> Self {
        Self { locators: Default::default() }
    }

    /// Returns the block height of the given peer IP.
    pub fn get_height(&self, peer_ip: &SocketAddr) -> Option<u32> {
        self.locators.read().get(peer_ip).map(|locators| locators.height())
    }

    /// Returns the list of peers with their heights, sorted by height (descending).
    pub fn get_peers_by_height(&self) -> Vec<(SocketAddr, u32)> {
        self.locators
            .read()
            .iter()
            .map(|(peer_ip, locators)| (*peer_ip, locators.height()))
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .collect()
    }

    /// Updates the block locators for the given peer IP.
    /// This function ensures all peers share a consistent view of the ledger.
    pub fn update_peer(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<()> {
        // Ensure the given block locators are well-formed.
        locators.ensure_is_valid()?;

        // Acquire the write lock on the locators map.
        let mut locators_write = self.locators.write();

        // Ensure the locators are consistent with the block locators of every peer (including itself).
        for (_, peer_locators) in locators_write.iter() {
            locators.ensure_is_consistent_with(peer_locators)?;
        }

        // Update the locators entry for the given peer IP.
        locators_write.entry(peer_ip).or_insert(locators);
        Ok(())
    }

    /// Removes the peer, if they exist.
    pub fn remove_peer(&self, peer_ip: SocketAddr) {
        // Remove the locators entry for the given peer IP.
        self.locators.write().remove(&peer_ip);
    }
}

/// TODO (howardwu): Move everything below here to a separate crate (maybe to ledger or messages).

/// The number of recent blocks (near tip).
pub const NUM_RECENTS: usize = 100; // 100 blocks
/// The interval between recent blocks.
pub const RECENT_INTERVAL: u32 = 1; // 1 block intervals
/// The interval between block checkpoints.
pub const CHECKPOINT_INTERVAL: u32 = 10_000; // 10,000 block intervals

#[derive(Clone, Debug)]
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
        // Ensure the last recent height is at or above the last checkpoint height.
        if last_recent_height < last_checkpoint_height {
            bail!("Recent height ({last_recent_height}) cannot be below checkpoint ({last_checkpoint_height})")
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
        for current_height in recents.keys() {
            if *current_height <= last_height {
                bail!("Recent blocks must increment in height")
            }
            if *current_height - last_height != RECENT_INTERVAL {
                bail!("Recent blocks must increment by {RECENT_INTERVAL}")
            }
            last_height = *current_height;
        }

        // If the last height is below NUM_RECENTS, ensure the number of recent blocks matches the last height.
        if last_height < NUM_RECENTS as u32 && recents.len() as u32 != last_height {
            bail!("As the last height is below {NUM_RECENTS}, the number of recent blocks must match the height")
        }
        // Otherwise, ensure the number of recent blocks matches NUM_RECENTS.
        else if last_height >= NUM_RECENTS as u32 && recents.len() != NUM_RECENTS {
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
            if *current_height - last_height != CHECKPOINT_INTERVAL {
                bail!("Block checkpoints must increment by {CHECKPOINT_INTERVAL}")
            }
            last_height = *current_height;
        }

        Ok(last_height)
    }
}

#[cfg(test)]
mod tests {}
