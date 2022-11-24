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

/// A tuple of the block hash (optional), previous block hash (optional), and peer IPs.
pub type RequestHashFromPeers<N> =
    (Option<<N as Network>::BlockHash>, Option<<N as Network>::BlockHash>, IndexSet<SocketAddr>);

#[derive(Clone, Debug)]
pub struct Sync<N: Network> {
    /// The canonical map of block height to block hash.
    /// This map is a linearly-increasing map of block heights to block hashes,
    /// updated solely from candidate blocks (not from block locators, to ensure there are no forks).
    canon: Arc<RwLock<BTreeMap<u32, N::BlockHash>>>,
    /// The map of peer IP to their block locators.
    /// This map is used to determine which blocks to request from peers.
    /// The block locators are consistent with the canonical map and every other peer's block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, BlockLocators<N>>>>,
    /// The map of block height to the expected block hash and peer IPs.
    /// Each entry is removed when its corresponding entry in the responses map is removed.
    requests: Arc<RwLock<BTreeMap<u32, RequestHashFromPeers<N>>>>,
    /// The map of block height to the received blocks.
    /// Removing an entry from this map must remove the corresponding entry from the requests map.
    responses: Arc<RwLock<BTreeMap<u32, Block<N>>>>,
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
            responses: Default::default(),
        }
    }

    /// Returns `true` if a request for the given block height exists.
    pub fn contains_request(&self, height: u32) -> bool {
        self.requests.read().contains_key(&height)
    }

    /// Returns the canonical block hash for the given block height, if it exists.
    pub fn get_canon_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.canon.read().get(&height).copied()
    }

    /// Returns the latest block height of the given peer IP.
    pub fn get_peer_height(&self, peer_ip: &SocketAddr) -> Option<u32> {
        self.locators.read().get(peer_ip).map(|locators| locators.latest_height())
    }

    /// Returns the list of peers with their heights, sorted by height (descending).
    pub fn get_sync_peers_by_height(&self) -> Vec<(SocketAddr, u32)> {
        self.locators
            .read()
            .iter()
            .map(|(peer_ip, locators)| (*peer_ip, locators.latest_height()))
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .collect()
    }

    /// Inserts a canonical block hash for the given block height, overriding an existing entry if it exists.
    pub fn insert_canon_locator(&self, height: u32, hash: N::BlockHash) {
        self.canon.write().insert(height, hash);
    }

    /// Inserts the block locators as canonical, overriding any existing entries.
    pub fn insert_canon_locators(&self, locators: BlockLocators<N>) -> Result<()> {
        // Ensure the given block locators are well-formed.
        locators.ensure_is_valid()?;
        // Insert the block locators into canon.
        for (height, hash) in locators.checkpoints.into_iter().chain(locators.recents.into_iter()) {
            self.canon.write().insert(height, hash);
        }
        Ok(())
    }

    /// Inserts a block request for the given height.
    pub fn insert_block_request(
        &self,
        height: u32,
        hash: Option<N::BlockHash>,
        previous_hash: Option<N::BlockHash>,
        peer_ips: IndexSet<SocketAddr>,
    ) -> Result<()> {
        // Ensure the block height is not already canon.
        if self.canon.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} exists in the canon map");
        }
        // Ensure the block height is not already requested.
        if self.requests.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} exists in the requests map");
        }
        // Ensure the block height is not already responded.
        if self.responses.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} exists in the responses map");
        }
        // Insert the block request.
        self.requests.write().insert(height, (hash, previous_hash, peer_ips));
        Ok(())
    }

    /// Inserts the given block response, after checking that the request exists and the response is well-formed.
    /// On success, this function removes the peer IP from the requests map.
    /// On failure, this function removes all block requests from the given peer IP.
    pub fn insert_block_response(&self, peer_ip: SocketAddr, block: Block<N>) -> Result<()> {
        // Retrieve the block height.
        let height = block.height();

        // Ensure the block height is not already canon. This should never happen.
        if self.canon.read().contains_key(&height) {
            error!("Failed to add block {height} (response) from '{peer_ip}', as it exists in the canon map");
            return Ok(());
        }

        // Ensure the block (response) from the peer is well-formed. On failure, remove all block requests to the peer.
        if let Err(error) = self.check_block_response(&peer_ip, &block) {
            // Remove all block requests to the peer.
            self.remove_block_requests(&peer_ip);
            return Err(error);
        }

        // Remove the peer IP from the request entry.
        self.remove_block_request(&peer_ip, height);

        // Acquire the write lock on the responses map.
        let mut responses = self.responses.write();
        // Insert the candidate block into the responses map.
        if let Some(existing_block) = responses.insert(height, block.clone()) {
            // If the candidate block was already present, ensure it is the same block.
            if block != existing_block {
                // Remove the candidate block.
                responses.remove(&height);
                // Remove all block requests to the peer.
                self.remove_block_requests(&peer_ip);
                bail!("Candidate block {height} from '{peer_ip}' is malformed");
            }
        }

        Ok(())
    }

    /// Updates the block locators for the given peer IP.
    /// This function ensures all peers share a consistent view of the ledger.
    /// On failure, this function returns a list of peer IPs to disconnect.
    pub fn update_peer_locators(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<(), Vec<SocketAddr>> {
        // If the locators match the existing locators for the peer, return early.
        if self.locators.read().get(&peer_ip) == Some(&locators) {
            return Ok(());
        }

        // Ensure the given block locators are valid. If not, remove any requests to the peer, and return early.
        if let Err(disconnect_ips) = self.check_locators(peer_ip, locators.clone()) {
            // Remove any requests to the peer.
            disconnect_ips.iter().for_each(|disconnect_ip| self.remove_block_requests(disconnect_ip));
            // Return the error.
            return Err(disconnect_ips);
        }

        // Update the locators entry for the given peer IP.
        self.locators.write().entry(peer_ip).or_insert(locators);

        Ok(())
    }

    /// Removes the block locators for the peer, if they exist.
    pub fn remove_peer_locators(&self, peer_ip: &SocketAddr) {
        // Remove the locators entry for the given peer IP.
        self.locators.write().remove(peer_ip);
    }

    /// Removes the block request for the given peer IP, if it exists.
    pub fn remove_block_request(&self, peer_ip: &SocketAddr, height: u32) {
        // Remove the peer IP from the request entry.
        if let Some((_, _, peer_ips)) = self.requests.write().get_mut(&height) {
            peer_ips.remove(peer_ip);
        }
    }

    /// Removes all block requests for the given peer IP.
    pub fn remove_block_requests(&self, peer_ip: &SocketAddr) {
        // Remove the peer IP from the requests map.
        self.requests.write().values_mut().for_each(|(_, _, peer_ips)| {
            peer_ips.remove(peer_ip);
        });
    }

    /// Removes and returns the block response for the given height, if the request is complete.
    pub fn remove_block_response(&self, height: u32) -> Option<Block<N>> {
        // Determine if the request is complete.
        let is_request_complete =
            self.requests.read().get(&height).map(|(_, _, peer_ips)| peer_ips.is_empty()).unwrap_or(false);

        // If the request is not complete, return early.
        if !is_request_complete {
            return None;
        }
        // Remove the request entry for the given height.
        self.requests.write().remove(&height);
        // Remove the response entry for the given height.
        self.responses.write().remove(&height)
    }
}

impl<N: Network> Sync<N> {
    /// Checks the given block (response) from a peer against the expected block hash and previous block hash.
    fn check_block_response(&self, peer_ip: &SocketAddr, block: &Block<N>) -> Result<()> {
        // Retrieve the block height.
        let height = block.height();

        // Retrieve the request entry for the candidate block.
        if let Some((expected_hash, expected_previous_hash, peer_ips)) = self.requests.read().get(&height) {
            // Ensure the candidate block hash matches the expected hash.
            if let Some(expected_hash) = expected_hash {
                if block.hash() != *expected_hash {
                    bail!("The block hash for candidate block {height} from '{peer_ip}' is incorrect")
                }
            }
            // Ensure the previous block hash matches if it exists.
            if let Some(expected_previous_hash) = expected_previous_hash {
                if block.previous_hash() != *expected_previous_hash {
                    bail!("The previous block hash in candidate block {height} from '{peer_ip}' is incorrect")
                }
            }
            // Ensure the sync pool requested this block from the given peer.
            if !peer_ips.contains(peer_ip) {
                bail!("The sync pool did not request block {height} from '{peer_ip}'")
            }
            Ok(())
        } else {
            bail!("The sync pool did not request block {height}")
        }
    }

    /// Checks the given block locators against the canonical map and block locators of all peers.
    /// This function ensures all peers share a consistent view of the ledger.
    /// On failure, this function returns a list of peer IPs to disconnect.
    fn check_locators(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<(), Vec<SocketAddr>> {
        // If the locators match the existing locators for the peer, return early.
        if self.locators.read().get(&peer_ip) == Some(&locators) {
            return Ok(());
        }

        // Ensure the given block locators are well-formed, or disconnect the peer.
        if let Err(error) = locators.ensure_is_valid() {
            warn!("Received invalid block locators from '{peer_ip}': {error}");
            return Err(vec![peer_ip]);
        }

        // Clone the canonical map.
        let canon = self.canon.read().clone();
        if !canon.is_empty() {
            // Iterate through every checkpoint and recent locator (which may include overlaps).
            locators.checkpoints.iter().chain(&locators.recents).try_for_each(|(height, hash)| {
                // Ensure the block locators are consistent with the canonical map.
                if let Some(canon_hash) = canon.get(height) {
                    // If the block locators are inconsistent, disconnect the peer.
                    if canon_hash != hash {
                        warn!("Received inconsistent block locators from '{peer_ip}'");
                        return Err(vec![peer_ip]);
                    }
                }
                Ok(())
            })?;
        }

        // Ensure the locators are consistent with the block locators of every peer (including itself).
        for (other_ip, other_locators) in self.locators.read().iter() {
            // If the locators are inconsistent, disconnect the peer.
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
}
