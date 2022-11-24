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
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use parking_lot::RwLock;
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

/// A tuple of the block hash, previous block hash (optional), and peer IPs.
pub type RequestHashFromPeers<N> = (<N as Network>::BlockHash, Option<<N as Network>::BlockHash>, IndexSet<SocketAddr>);

#[derive(Clone, Debug)]
pub struct Sync<N: Network> {
    /// The canonical map of block height to block hash.
    /// This map is a linearly-increasing map of block heights to block hashes,
    /// updated solely from candidate blocks (not from block locators, to ensure there are no forks).
    canon: Arc<RwLock<BTreeMap<u32, N::BlockHash>>>,
    /// The map of peer IP to their block locators.
    /// This map is updated from block locators, and is used to determine which blocks to request from peers.
    /// The block locators are guaranteed to be consistent with the canonical map,
    /// and with every other peer's block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, BlockLocators<N>>>>,

    /// The map of block height to the expected block hash and peer IPs.
    requests: Arc<RwLock<BTreeMap<u32, RequestHashFromPeers<N>>>>,
    /// The map of block height to the received blocks.
    candidates: Arc<RwLock<BTreeMap<u32, Block<N>>>>,
    /// The map of block height to the success blocks.
    success: Arc<RwLock<BTreeMap<u32, Block<N>>>>,
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
        Self {
            canon: Default::default(),
            locators: Default::default(),
            requests: Default::default(),
            candidates: Default::default(),
            success: Default::default(),
        }
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

    /// Inserts a block request for the given height.
    pub fn insert_request(
        &self,
        height: u32,
        hash: N::BlockHash,
        previous_hash: Option<N::BlockHash>,
        peer_ips: IndexSet<SocketAddr>,
    ) -> Result<()> {
        // Ensure the block height is not already canon.
        if self.canon.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} already exists in the canon map");
        }
        // Ensure the block height is not already requested.
        if self.requests.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} already exists in the requests map");
        }
        // Ensure the block height is not already a candidate.
        if self.candidates.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} already exists in the candidates map");
        }
        // Ensure the block height is not already a success.
        if self.success.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} already exists in the success map");
        }
        // Insert the block request.
        self.requests.write().insert(height, (hash, previous_hash, peer_ips));
        Ok(())
    }

    /// Inserts the given candidate block, after checking that the request was made, and the
    /// expected block hash matches. On success, this function also removes the peer IP from the requests map.
    pub fn insert_candidate_block(&self, peer_ip: SocketAddr, block: Block<N>) -> Result<()> {
        let candidate_height = block.height();

        // Ensure the canonical map does not contain the candidate block.
        if self.canon.read().contains_key(&candidate_height) {
            bail!("The canonical map already contains the candidate block")
        }

        // Declare a boolean flag to determine if the request is complete.
        let is_request_complete;

        // Retrieve the request entry for the candidate block.
        if let Some((expected_hash, expected_previous_hash, peer_ips)) =
            self.requests.write().get_mut(&candidate_height)
        {
            // Ensure the sync pool requested this block from the given peer.
            if !peer_ips.contains(&peer_ip) {
                bail!("The sync pool did not request block {candidate_height} from '{peer_ip}'")
            }
            // Ensure the candidate block hash matches the expected hash.
            if block.hash() != *expected_hash {
                bail!("The block hash for candidate block {candidate_height} from '{peer_ip}' is incorrect")
            }
            // Ensure the previous block hash matches if it exists.
            if let Some(expected_previous_hash) = expected_previous_hash {
                if block.previous_hash() != *expected_previous_hash {
                    bail!("The previous block hash in candidate block {candidate_height} from '{peer_ip}' is incorrect")
                }
            }
            // Remove the request entry for this peer IP.
            peer_ips.remove(&peer_ip);
            // Update whether the request is complete.
            is_request_complete = peer_ips.is_empty();
        } else {
            bail!("The sync pool did not request block {candidate_height}")
        }

        // Acquire a write lock on the candidates map.
        let mut candidates_write = self.candidates.write();

        // Insert the candidate block into the candidates map.
        if let Some(existing_block) = candidates_write.insert(candidate_height, block.clone()) {
            // If the candidate block was already present, ensure it is the same block.
            if block != existing_block {
                // If the candidate block is different, remove this entry from the candidates map.
                candidates_write.remove(&candidate_height);
                bail!("Candidate block {candidate_height} is malformed");
            }
        }

        // If there are no more peer IPs for this request, remove the request entry,
        // and move the candidate block to the success map and update the canonical map.
        if is_request_complete {
            // Remove the request entry.
            self.requests.write().remove(&candidate_height);
            // Move the candidate block to the success map and update the canonical map.
            if let Some(block) = candidates_write.remove(&candidate_height) {
                self.canon.write().insert(block.height(), block.hash());
                self.success.write().insert(block.height(), block);
            }
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

    /// Removes the successful block for the given height, returning the block if it exists.
    pub fn remove_successful_block(&self, height: u32) -> Option<Block<N>> {
        // Remove the success entry for the given height.
        self.success.write().remove(&height)
    }
}
