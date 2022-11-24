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
use snarkvm::prelude::{Block, Network};

use anyhow::{bail, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use parking_lot::RwLock;
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub struct Sync<N: Network> {
    /// The canonical map of block heights to block hashes.
    /// This map is a linearly-increasing map of block heights to block hashes,
    /// updated solely from candidate blocks (not from block locators, to ensure there are no forks).
    canon: Arc<RwLock<BTreeMap<u32, N::BlockHash>>>,
    /// The map of peer IPs to their block locators.
    /// This map is updated from block locators, and is used to determine which blocks to request from peers.
    /// The block locators are guaranteed to be consistent with the canonical map,
    /// and with every other peer's block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, BlockLocators<N>>>>,
    /// The map of block requests to the received blocks.
    candidates: Arc<RwLock<BTreeMap<u32, Block<N>>>>,
}

impl<N: Network> Default for Sync<N> {
    /// Initializes a new instance of the sync pool.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Sync<N> {
    /// Initializes a new instance of the sync pool.
    pub fn new() -> Self {
        Self { canon: Default::default(), locators: Default::default(), candidates: Default::default() }
    }

    /// Returns the canonical block hash for the given block height, if it exists.
    pub fn get_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.canon.read().get(&height).copied()
    }

    /// Returns the latest block height of the given peer IP.
    pub fn get_peer_height(&self, peer_ip: &SocketAddr) -> Option<u32> {
        self.locators.read().get(peer_ip).map(|locators| locators.latest_height())
    }

    /// Returns the list of peers with their heights, sorted by height (descending).
    pub fn get_peers_by_height(&self) -> Vec<(SocketAddr, u32)> {
        self.locators
            .read()
            .iter()
            .map(|(peer_ip, locators)| (*peer_ip, locators.latest_height()))
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .collect()
    }

    /// Inserts the candidate blocks.
    pub fn insert_candidate_blocks(&self, peer_ip: SocketAddr, candidate_blocks: Vec<Block<N>>) -> Result<()> {
        // Retrieve the block locators for the given peer IP.
        let locators = match self.locators.read().get(&peer_ip) {
            Some(locators) => locators.clone(),
            None => bail!("Missing block locators for '{peer_ip}'"),
        };

        let mut candidates = self.candidates.write();
        for block in candidate_blocks {
            // Check the candidate block hash against the canonical map.
            if let Some(hash) = self.canon.read().get(&block.height()) {
                if hash != &block.hash() {
                    bail!("Received a block from '{peer_ip}' with an incorrect block hash (canon mismatch)");
                }
            }
            // Check the candidate block hash against the known block locators of this peer.
            if let Some(hash) = locators.get_hash(block.height()) {
                if hash != block.hash() {
                    bail!("Received a block from '{peer_ip}' with an incorrect block hash (locator mismatch)");
                }
            }
            // If the candidate block height already exists, check that the candidate block hash is the same.
            if let Some(candidate_block) = candidates.get(&block.height()) {
                if candidate_block.hash() != block.hash() {
                    bail!("Received a block from '{peer_ip}' with an incorrect block hash (candidate mismatch)");
                }
            }
            // Update the canonical map.
            self.canon.write().insert(block.height(), block.hash());
            // Insert the candidate block.
            candidates.insert(block.height(), block);
        }
        Ok(())
    }

    /// Checks the given block locators against the canonical map and block locators of all peers.
    /// This function ensures all peers share a consistent view of the ledger.
    /// On failure, this function returns a list of peer IPs to disconnect.
    pub fn check_locators(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<(), Vec<SocketAddr>> {
        // If the locators match the existing locators for the peer, return early.
        if self.locators.read().get(&peer_ip) == Some(&locators) {
            return Ok(());
        }

        // Ensure the given block locators are well-formed.
        if let Err(error) = locators.ensure_is_valid() {
            warn!("Received invalid block locators from '{peer_ip}': {error}");
            return Err(vec![peer_ip]);
        }

        // Ensure the block locators are consistent with the canonical map.
        let canon = self.canon.read();
        if !canon.is_empty() {
            for (height, hash) in locators.checkpoints.iter().chain(locators.recents.iter()) {
                if let Some(canon_hash) = canon.get(height) {
                    if canon_hash != hash {
                        warn!("'{peer_ip}' has an inconsistent block hash at block {height}");
                        return Err(vec![peer_ip]);
                    }
                }
            }
        }
        drop(canon);

        // Ensure the locators are consistent with the block locators of every peer (including itself).
        for (other_ip, other_locators) in self.locators.read().iter() {
            if let Err(error) = locators.ensure_is_consistent_with(other_locators) {
                warn!("Inconsistent block locators between '{peer_ip}' and '{other_ip}': {error}");
                match peer_ip == *other_ip {
                    true => return Err(vec![peer_ip]),
                    false => return Err(vec![peer_ip, *other_ip]),
                }
            }
        }

        Ok(())
    }

    /// Updates the block locators for the given peer IP.
    /// This function ensures all peers share a consistent view of the ledger.
    /// On failure, this function returns a list of peer IPs to disconnect.
    pub fn update_locators(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<(), Vec<SocketAddr>> {
        // If the locators match the existing locators for the peer, return early.
        if self.locators.read().get(&peer_ip) == Some(&locators) {
            return Ok(());
        }

        // Ensure the given block locators are valid.
        self.check_locators(peer_ip, locators.clone())?;
        // Update the locators entry for the given peer IP.
        self.locators.write().entry(peer_ip).or_insert(locators);

        Ok(())
    }

    /// Removes the block locators for the peer, if they exist.
    pub fn remove_peer(&self, peer_ip: &SocketAddr) {
        // Remove the locators entry for the given peer IP.
        self.locators.write().remove(peer_ip);
    }
}
