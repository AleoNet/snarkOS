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

use snarkos_node_messages::BlockLocators;
use snarkvm::prelude::Network;

use anyhow::Result;
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
    pub fn remove_peer(&self, peer_ip: &SocketAddr) {
        // Remove the locators entry for the given peer IP.
        self.locators.write().remove(peer_ip);
    }
}
