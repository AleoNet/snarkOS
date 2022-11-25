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

use snarkos_node_messages::{BlockLocators, BlockRequest, DataBlocks};
use snarkvm::prelude::{Block, Network};

use anyhow::{bail, Result};
use colored::Colorize;
use core::hash::Hash;
use futures::TryStreamExt;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use parking_lot::RwLock;
use rand::{
    prelude::{IteratorRandom, SliceRandom},
    CryptoRng,
    Rng,
};
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

pub const REDUNDANCY_FACTOR: usize = 3;
pub const EXTRA_REDUNDANCY_FACTOR: usize = REDUNDANCY_FACTOR * 2;
pub const NUM_SYNC_CANDIDATE_PEERS: usize = REDUNDANCY_FACTOR * 5;

/// A tuple of the block hash (optional), previous block hash (optional), and peer IPs.
pub type RequestHashFromPeers<N> =
    (Option<<N as Network>::BlockHash>, Option<<N as Network>::BlockHash>, IndexSet<SocketAddr>);

#[derive(Copy, Clone, Debug)]
pub struct PeerPair(SocketAddr, SocketAddr);

impl Eq for PeerPair {}

impl PartialEq for PeerPair {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl Hash for PeerPair {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (a, b) = if self.0 < self.1 { (self.0, self.1) } else { (self.1, self.0) };
        a.hash(state);
        b.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct Sync<N: Network> {
    /// The listener IP of this node.
    local_ip: SocketAddr,
    /// The canonical map of block height to block hash.
    /// This map is a linearly-increasing map of block heights to block hashes,
    /// updated solely from candidate blocks (not from block locators, to ensure there are no forks).
    canon: Arc<RwLock<BTreeMap<u32, N::BlockHash>>>,
    /// The map of peer IP to their block locators.
    /// The block locators are consistent with the canonical map and every other peer's block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, BlockLocators<N>>>>,
    /// The map of peer-to-peer to their common ancestor.
    /// This map is used to determine which peers to request blocks from.
    common_ancestors: Arc<RwLock<IndexMap<PeerPair, u32>>>,
    /// The map of block height to the expected block hash and peer IPs.
    /// Each entry is removed when its corresponding entry in the responses map is removed.
    requests: Arc<RwLock<BTreeMap<u32, RequestHashFromPeers<N>>>>,
    /// The map of block height to the received blocks.
    /// Removing an entry from this map must remove the corresponding entry from the requests map.
    responses: Arc<RwLock<BTreeMap<u32, Block<N>>>>,
}

impl<N: Network> Sync<N> {
    /// Initializes a new instance of the sync pool.
    pub fn new(local_ip: SocketAddr) -> Self {
        Self {
            local_ip,
            canon: Default::default(),
            locators: Default::default(),
            common_ancestors: Default::default(),
            requests: Default::default(),
            responses: Default::default(),
        }
    }

    /// Returns `true` if a request for the given block height exists.
    pub fn contains_request(&self, height: u32) -> bool {
        self.requests.read().contains_key(&height)
    }

    /// Returns the latest block height in the sync pool.
    pub fn latest_canon_height(&self) -> u32 {
        self.canon.read().keys().last().copied().unwrap_or(0)
    }

    /// Returns the canonical block height, if it exists.
    pub fn get_canon_height(&self, hash: &N::BlockHash) -> Option<u32> {
        self.canon.read().iter().find(|(_, h)| h == &hash).map(|(h, _)| *h)
    }

    /// Returns the canonical block hash for the given block height, if it exists.
    pub fn get_canon_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.canon.read().get(&height).copied()
    }

    /// Returns the latest block height of the given peer IP.
    pub fn get_peer_height(&self, peer_ip: &SocketAddr) -> Option<u32> {
        self.locators.read().get(peer_ip).map(|locators| locators.latest_locator_height())
    }

    /// Returns a map of peer height to peer IPs.
    /// e.g. `{{ 127 => \[peer1, peer2\], 128 => \[peer3\], 135 => \[peer4, peer5\] }}`
    pub fn get_peer_heights(&self) -> BTreeMap<u32, Vec<SocketAddr>> {
        self.locators.read().iter().map(|(peer_ip, locators)| (locators.latest_locator_height(), *peer_ip)).fold(
            Default::default(),
            |mut map, (height, peer_ip)| {
                map.entry(height).or_insert_with(Vec::new).push(peer_ip);
                map
            },
        )
    }

    /// Returns the list of peers with their heights, sorted by height (descending).
    pub fn get_peers_by_height(&self) -> Vec<(SocketAddr, u32)> {
        self.locators
            .read()
            .iter()
            .map(|(peer_ip, locators)| (*peer_ip, locators.latest_locator_height()))
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .collect()
    }

    /// Returns a list of block requests, if the node needs to sync.
    pub fn prepare_block_requests(
        &self,
    ) -> Vec<(u32, (Option<N::BlockHash>, Option<N::BlockHash>, IndexSet<SocketAddr>))> {
        // Retrieve the latest canon height.
        let latest_canon_height = self.latest_canon_height();

        // Pick a set of peers above the latest canon height, and include their heights.
        let candidate_peers: IndexMap<_, _> = self
            .locators
            .read()
            .iter()
            .map(|(peer_ip, locators)| (*peer_ip, locators.latest_locator_height()))
            .filter(|(_, height)| *height > latest_canon_height)
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .take(NUM_SYNC_CANDIDATE_PEERS)
            .collect();

        // Case 0a: If there are no candidate peers, return `None`.
        if candidate_peers.is_empty() {
            trace!("No sync peers (this node is ahead)");
            return vec![];
        }

        // Retrieve the common ancestors of the peers above the latest canon height.
        let mut common_ancestors: IndexSet<_> = self
            .common_ancestors
            .read()
            .iter()
            .filter(|(peer_pair, _)| {
                candidate_peers.contains_key(&peer_pair.0) || candidate_peers.contains_key(&peer_pair.1)
            })
            .map(|(_, height)| *height)
            .collect();

        // Case 0b: If there are no common ancestors between the candidate peers, return `None`.
        // (We likely just started the node, and are waiting for the first block locators.)
        if common_ancestors.is_empty() {
            trace!("No sync peers (waiting for block locators)");
            return vec![];
        }

        // Clone the locators.
        let locators = self.locators.read().clone();

        let min_common_ancestor = common_ancestors.iter().min().copied().unwrap_or_default();

        let (end_peer_height, sync_peers) =
            // Case 1: If all of the candidate peers share a common ancestor at or above the latest canon height,
            // then pick peer(s) at random.
            //
            // This case is typically triggered when syncing from genesis.
            if min_common_ancestor >= latest_canon_height {
                println!("Case 1");
                (min_common_ancestor, candidate_peers)
            }
            // TODO (howardwu): Change this to the highest cumulative weight for Phase 3.
            // Case 2: If all of the candidate peers share a common ancestor below the latest canon height,
            // then pick the peer with the highest height, and find peers (up to extra redundancy) with
            // a common ancestor above the block request range. Set the end height to their common ancestor.
            else {
                println!("Case 2");
                let mut end_peer_height = 0;
                let mut sync_peers = IndexMap::new();

                for (i, (peer_ip, peer_height)) in candidate_peers.iter().enumerate() {
                    // As the previous iteration did not `break`, reset the end peer height and clear the sync peers.
                    end_peer_height = 0;
                    sync_peers.clear();

                    // Retrieve the block locators for this peer.
                    let peer_locators = match locators.get(peer_ip) {
                        Some(locators) => locators,
                        None => continue,
                    };

                    // Set the end peer height.
                    end_peer_height = *peer_height;
                    // Add the peer to the sync peers.
                    sync_peers.insert(*peer_ip, *peer_height);

                    for (other_ip, other_height) in candidate_peers.iter().skip(i + 1) {
                        // Check if these two peers have a common ancestor above the latest canon height.
                        if let Some(common_ancestor) = self.common_ancestors.read().get(&PeerPair(*peer_ip, *other_ip)) {
                            if *common_ancestor > latest_canon_height {
                                // If so, then check that their block locators are consistent.
                                if let Some(other_locators) = locators.get(other_ip) {
                                    if peer_locators.is_consistent_with(other_locators) {
                                        // If the common ancestor is less than the end peer height, then set the end peer height.
                                        if *common_ancestor < end_peer_height {
                                            end_peer_height = *common_ancestor;
                                        }
                                        // Add the other peer to the list of sync peers.
                                        sync_peers.insert(*other_ip, *other_height);
                                    }
                                }
                            }
                        }
                    }

                    // If we have enough sync peers at or above the latest canon height, then break the loop.
                    if end_peer_height >= latest_canon_height && sync_peers.len() >= REDUNDANCY_FACTOR {
                        break;
                    }
                }

                // If there is no cohort of peers with a common ancestor above the latest canon height,
                // then return early.
                if end_peer_height < latest_canon_height || sync_peers.is_empty() {
                    return vec![];
                }

                (end_peer_height, sync_peers)
            };

        // Initialize an RNG.
        let rng = &mut rand::thread_rng();

        // Compute the start height for the block request.
        let start_height = latest_canon_height + 1;
        // Compute the end height for the block request.
        let end_height = end_peer_height.min(start_height + DataBlocks::<N>::MAXIMUM_NUMBER_OF_BLOCKS as u32) + 1;

        let mut current_height = start_height;
        let mut requests = Vec::with_capacity((end_height - current_height) as usize);

        while current_height <= end_height {
            // Ensure the current height does not contain block requests.
            if self.contains_request(current_height) {
                // Increment the current height by 1.
                current_height += 1;
                continue;
            }

            // Determine the peer IPs that have this block.
            let peer_ips = sync_peers
                .iter()
                .filter(|(_, height)| **height >= current_height)
                .map(|(peer_ip, _)| *peer_ip)
                .collect::<Vec<_>>();
            // Append the request.
            requests.push((current_height, construct_request(current_height, &peer_ips, &locators, rng)));
            // Increment the current height by 1.
            current_height += 1;
        }

        return requests;

        // // Determine the number of block requests to make.
        // let num_block_requests = 1 + (min_common_ancestor - latest_canon_height) / DataBlocks::MAXIMUM_NUMBER_OF_BLOCKS as u32;
        //
        // // Determine the list of block requests.
        // let block_requests = (0..num_block_requests)
        //     .map(|i| {
        //         let start_height = 1 + latest_canon_height + i * DataBlocks::MAXIMUM_NUMBER_OF_BLOCKS as u32;
        //         let end_height = 1 + min_common_ancestor.min(start_height + DataBlocks::MAXIMUM_NUMBER_OF_BLOCKS as u32);
        //         BlockRequest {
        //             start_height,
        //             end_height,
        //         }
        //     })
        //     .collect();
    }

    /// Inserts a canonical block hash for the given block height, overriding an existing entry if it exists.
    pub fn insert_canon_locator(&self, height: u32, hash: N::BlockHash) {
        if let Some(previous_hash) = self.canon.write().insert(height, hash) {
            // Warn if this insert overrides a different previous block hash.
            if previous_hash != hash {
                let change = format!("(from {previous_hash} to {hash})").dimmed();
                warn!("Sync pool overrode the canon block hash at block {height} {change}");
            }
        }
    }

    /// Inserts the block locators as canonical, overriding any existing entries.
    pub fn insert_canon_locators(&self, locators: BlockLocators<N>) -> Result<()> {
        // Ensure the given block locators are well-formed.
        locators.ensure_is_valid()?;
        // Insert the block locators into canon.
        locators.checkpoints.into_iter().chain(locators.recents.into_iter()).for_each(|(height, hash)| {
            self.insert_canon_locator(height, hash);
        });
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
        // Ensure the block request does not already exist.
        self.check_block_request(height)?;
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

    /// Updates the block locators and common ancestors for the given peer IP.
    /// This function checks that the given block locators are well-formed, however it does **not** check
    /// that the block locators are consistent the peer's previous block locators or other peers' block locators.
    pub fn update_peer_locators(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<()> {
        // If the locators match the existing locators for the peer, return early.
        if self.locators.read().get(&peer_ip) == Some(&locators) {
            return Ok(());
        }

        // Ensure the given block locators are well-formed.
        locators.ensure_is_valid()?;
        // Update the locators entry for the given peer IP.
        self.locators.write().entry(peer_ip).or_insert(locators.clone());

        // Compute the common ancestor with this node.
        let mut ancestor = 0;
        for (height, hash) in locators.clone().into_iter() {
            match self.get_canon_hash(height) == Some(hash) {
                true => ancestor = height,
                false => break,
            }
        }
        // Update the common ancestor entry for this node.
        self.common_ancestors.write().entry(PeerPair(self.local_ip, peer_ip)).or_insert(ancestor);

        // Compute the common ancestor with every other peer.
        let mut common_ancestors = self.common_ancestors.write();
        for (other_ip, other_locators) in self.locators.read().iter() {
            // Skip if the other peer is the given peer.
            if other_ip == &peer_ip {
                continue;
            }
            // Compute the common ancestor with the other peer.
            let mut ancestor = 0;
            for (height, hash) in other_locators.clone().into_iter() {
                match locators.get_hash(height) == Some(hash) {
                    true => ancestor = height,
                    false => break,
                }
            }
            common_ancestors.insert(PeerPair(peer_ip, *other_ip), ancestor);
        }

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
    /// Checks that a block request for the given height does not already exist.
    fn check_block_request(&self, height: u32) -> Result<()> {
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
        Ok(())
    }

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
        // // Ensure the given block locators are valid. If not, remove any requests to the peer, and return early.
        // if let Err(disconnect_ips) = self.check_locators(peer_ip, locators.clone()) {
        //     // Remove any requests to the peer.
        //     disconnect_ips.iter().for_each(|disconnect_ip| self.remove_block_requests(disconnect_ip));
        //     // Return the error.
        //     return Err(disconnect_ips);
        // }

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
            // Iterate through every checkpoint and recent locator.
            locators.clone().into_iter().try_for_each(|(height, hash)| {
                // Ensure the block locators are consistent with the canonical map.
                if let Some(canon_hash) = canon.get(&height) {
                    // If the block locators are inconsistent, disconnect the peer.
                    if canon_hash != &hash {
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

/// If any peer is detected to be dishonest in this function, it will not set the hash or previous hash,
/// in order to allow the caller to determine what to do.
fn construct_request<N: Network, R: Rng + CryptoRng>(
    height: u32,
    peer_ips: &[SocketAddr],
    locators: &IndexMap<SocketAddr, BlockLocators<N>>,
    rng: &mut R,
) -> (Option<N::BlockHash>, Option<N::BlockHash>, IndexSet<SocketAddr>) {
    let mut hash = None;
    let mut hash_redundancy: usize = 0;
    let mut previous_hash = None;
    let mut previous_hash_redundancy: usize = 0;
    let mut is_honest = true;

    for peer_ip in peer_ips {
        if let Some(locators) = locators.get(peer_ip) {
            if let Some(candidate_hash) = locators.get_hash(height) {
                match hash {
                    // Increment the redundancy count if the hash matches.
                    Some(hash) if hash == candidate_hash => hash_redundancy += 1,
                    // Some peer is dishonest.
                    Some(_) => {
                        hash = None;
                        hash_redundancy = 0;
                        previous_hash = None;
                        previous_hash_redundancy = 0;
                        is_honest = false;
                        break;
                    }
                    // Set the hash if it is not set.
                    None => {
                        hash = Some(candidate_hash);
                        hash_redundancy = 1;
                    }
                }
            }
            if let Some(candidate_previous_hash) = locators.get_hash(height.saturating_sub(1)) {
                match previous_hash {
                    // Increment the redundancy count if the previous hash matches.
                    Some(previous_hash) if previous_hash == candidate_previous_hash => previous_hash_redundancy += 1,
                    // Some peer is dishonest.
                    Some(_) => {
                        hash = None;
                        hash_redundancy = 0;
                        previous_hash = None;
                        previous_hash_redundancy = 0;
                        is_honest = false;
                        break;
                    }
                    // Set the previous hash if it is not set.
                    None => {
                        previous_hash = Some(candidate_previous_hash);
                        previous_hash_redundancy = 1;
                    }
                }
            }
        }
    }

    // Pick the sync IPs.
    let sync_ips =
        // Extra redundant peers - as the block hash was dishonest.
        if !is_honest {
            // TODO (howardwu): Consider performing an integrity check on peers (to disconnect).
            warn!("Detected dishonest peer(s) when preparing block request");
            // Choose up to the extra redundancy factor in sync peers.
            peer_ips.iter().copied().choose_multiple(rng, EXTRA_REDUNDANCY_FACTOR)
        }
        // No redundant peers - as we have redundancy on the block hash.
        else if hash.is_some() && hash_redundancy >= REDUNDANCY_FACTOR {
            // Choose a sync peer.
            peer_ips.iter().copied().choose_multiple(rng, 1)
        }
        // Redundant peers - as we do not have redundancy on the block hash.
        else {
            // Choose up to the redundancy factor in sync peers.
            peer_ips.iter().copied().choose_multiple(rng, REDUNDANCY_FACTOR)
        };

    // Return the request.
    (hash, previous_hash, sync_ips.into_iter().collect())
}
