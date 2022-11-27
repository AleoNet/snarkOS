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

use snarkos_node_messages::{BlockLocators, DataBlocks};
use snarkvm::prelude::{Block, Network};

use anyhow::{bail, ensure, Result};
use colored::Colorize;
use core::hash::Hash;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use parking_lot::RwLock;
use rand::{prelude::IteratorRandom, CryptoRng, Rng};
use std::{collections::BTreeMap, net::SocketAddr, ops::Range, sync::Arc};

pub const REDUNDANCY_FACTOR: usize = 3;
pub const EXTRA_REDUNDANCY_FACTOR: usize = REDUNDANCY_FACTOR * 2;
pub const NUM_SYNC_CANDIDATE_PEERS: usize = REDUNDANCY_FACTOR * 5;

/// A tuple of the block hash (optional), previous block hash (optional), and sync IPs.
pub type SyncRequest<N> = (Option<<N as Network>::BlockHash>, Option<<N as Network>::BlockHash>, IndexSet<SocketAddr>);

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
    /// updated solely from the ledger and candidate blocks (not from peers' block locators, to ensure there are no forks).
    canon: Arc<RwLock<BTreeMap<u32, N::BlockHash>>>,
    /// The map of peer IP to their block locators.
    /// The block locators are consistent with the canonical map and every other peer's block locators.
    locators: Arc<RwLock<IndexMap<SocketAddr, BlockLocators<N>>>>,
    /// The map of peer-to-peer to their common ancestor.
    /// This map is used to determine which peers to request blocks from.
    common_ancestors: Arc<RwLock<IndexMap<PeerPair, u32>>>,
    /// The map of block height to the expected block hash and peer IPs.
    /// Each entry is removed when its corresponding entry in the responses map is removed.
    requests: Arc<RwLock<BTreeMap<u32, SyncRequest<N>>>>,
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

    /// Returns the common ancestor for the given peer pair, if it exists.
    pub fn get_common_ancestor(&self, peer_a: SocketAddr, peer_b: SocketAddr) -> Option<u32> {
        self.common_ancestors.read().get(&PeerPair(peer_a, peer_b)).copied()
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
    pub fn insert_block_request(&self, height: u32, (hash, previous_hash, sync_ips): SyncRequest<N>) -> Result<()> {
        // Ensure the block request does not already exist.
        self.check_block_request(height)?;
        // Ensure the sync IPs are not empty.
        ensure!(!sync_ips.is_empty(), "Cannot insert a block request with no sync IPs");
        // Insert the block request.
        self.requests.write().insert(height, (hash, previous_hash, sync_ips));
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
        self.locators.write().insert(peer_ip, locators.clone());

        // Compute the common ancestor with this node.
        let mut ancestor = 0;
        for (height, hash) in locators.clone().into_iter() {
            if let Some(canon_hash) = self.get_canon_hash(height) {
                match canon_hash == hash {
                    true => ancestor = height,
                    false => break, // fork
                }
            }
        }
        // Update the common ancestor entry for this node.
        self.common_ancestors.write().insert(PeerPair(self.local_ip, peer_ip), ancestor);

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
                if let Some(expected_hash) = locators.get_hash(height) {
                    match expected_hash == hash {
                        true => ancestor = height,
                        false => break, // fork
                    }
                }
            }
            common_ancestors.insert(PeerPair(peer_ip, *other_ip), ancestor);
        }

        Ok(())
    }

    /// Removes the block locators for the peer, if they exist.
    pub fn remove_peer(&self, peer_ip: &SocketAddr) {
        // Remove the locators entry for the given peer IP.
        self.locators.write().remove(peer_ip);
        // Remove all block requests to the peer.
        self.remove_block_requests(peer_ip);
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

    // /// Checks the given block locators against the canonical map and block locators of all peers.
    // /// This function ensures all peers share a consistent view of the ledger.
    // /// On failure, this function returns a list of peer IPs to disconnect.
    // fn check_locators(&self, peer_ip: SocketAddr, locators: BlockLocators<N>) -> Result<(), Vec<SocketAddr>> {
    //     // // Ensure the given block locators are valid. If not, remove any requests to the peer, and return early.
    //     // if let Err(disconnect_ips) = self.check_locators(peer_ip, locators.clone()) {
    //     //     // Remove any requests to the peer.
    //     //     disconnect_ips.iter().for_each(|disconnect_ip| self.remove_block_requests(disconnect_ip));
    //     //     // Return the error.
    //     //     return Err(disconnect_ips);
    //     // }
    //
    //     // If the locators match the existing locators for the peer, return early.
    //     if self.locators.read().get(&peer_ip) == Some(&locators) {
    //         return Ok(());
    //     }
    //
    //     // Ensure the given block locators are well-formed, or disconnect the peer.
    //     if let Err(error) = locators.ensure_is_valid() {
    //         warn!("Received invalid block locators from '{peer_ip}': {error}");
    //         return Err(vec![peer_ip]);
    //     }
    //
    //     // Clone the canonical map.
    //     let canon = self.canon.read().clone();
    //     if !canon.is_empty() {
    //         // Iterate through every checkpoint and recent locator.
    //         locators.clone().into_iter().try_for_each(|(height, hash)| {
    //             // Ensure the block locators are consistent with the canonical map.
    //             if let Some(canon_hash) = canon.get(&height) {
    //                 // If the block locators are inconsistent, disconnect the peer.
    //                 if canon_hash != &hash {
    //                     warn!("Received inconsistent block locators from '{peer_ip}'");
    //                     return Err(vec![peer_ip]);
    //                 }
    //             }
    //             Ok(())
    //         })?;
    //     }
    //
    //     // Ensure the locators are consistent with the block locators of every peer (including itself).
    //     for (other_ip, other_locators) in self.locators.read().iter() {
    //         // If the locators are inconsistent, disconnect the peer.
    //         if let Err(error) = locators.ensure_is_consistent_with(other_locators) {
    //             warn!("Inconsistent block locators between '{peer_ip}' and '{other_ip}': {error}");
    //             match peer_ip == *other_ip {
    //                 true => return Err(vec![peer_ip]),
    //                 false => return Err(vec![peer_ip, *other_ip]),
    //             }
    //         }
    //     }
    //
    //     Ok(())
    // }

    /// Returns a list of block requests, if the node needs to sync.
    pub fn prepare_block_requests(&self) -> Vec<(u32, SyncRequest<N>)> {
        // Retrieve the latest canon height.
        let latest_canon_height = self.latest_canon_height();

        // Pick a set of peers above the latest canon height, and include their locators.
        let candidate_locators: IndexMap<_, _> = self
            .locators
            .read()
            .iter()
            .filter(|(_, locators)| locators.latest_locator_height() > latest_canon_height)
            .sorted_by(|(_, a), (_, b)| b.latest_locator_height().cmp(&a.latest_locator_height()))
            .take(NUM_SYNC_CANDIDATE_PEERS)
            .map(|(peer_ip, locators)| (*peer_ip, locators.clone()))
            .collect();

        // Case 0a: If there are no candidate peers, return `None`.
        if candidate_locators.is_empty() {
            return vec![];
        }

        // TODO (howardwu): Change this to the highest cumulative weight for Phase 3.
        // Case 1: If all of the candidate peers share a common ancestor below the latest canon height,
        // then pick the peer with the highest height, and find peers (up to extra redundancy) with
        // a common ancestor above the block request range. Set the end height to their common ancestor.

        // Determine the threshold number of peers to sync from.
        let threshold_to_request = core::cmp::min(candidate_locators.len(), REDUNDANCY_FACTOR);
        println!("Case 1 - candidates: {:?}, threshold_to_request: {threshold_to_request}", candidate_locators.keys());

        let mut min_common_ancestor = 0;
        let mut sync_peers = IndexMap::new();

        // Breaks the loop when the first threshold number of peers are found, biasing for the peer with the highest height
        // and a cohort of peers who share a common ancestor above this node's latest canon height.
        for (i, (peer_ip, peer_locators)) in candidate_locators.iter().enumerate() {
            // As the previous iteration did not `break`, reset the sync peers.
            sync_peers.clear();

            // Set the minimum common ancestor.
            min_common_ancestor = peer_locators.latest_locator_height();
            // Add the peer to the sync peers.
            sync_peers.insert(*peer_ip, peer_locators.latest_locator_height());

            for (other_ip, other_locators) in candidate_locators.iter().skip(i + 1) {
                // Check if these two peers have a common ancestor above the latest canon height.
                if let Some(common_ancestor) = self.common_ancestors.read().get(&PeerPair(*peer_ip, *other_ip)) {
                    if *common_ancestor > latest_canon_height {
                        // If so, then check that their block locators are consistent.
                        if peer_locators.is_consistent_with(other_locators) {
                            // If their common ancestor is less than the minimum common ancestor, then update it.
                            if *common_ancestor < min_common_ancestor {
                                min_common_ancestor = *common_ancestor;
                            }
                            // Add the other peer to the list of sync peers.
                            sync_peers.insert(*other_ip, other_locators.latest_locator_height());
                        }
                    }
                }
            }

            // If we have enough sync peers above the latest canon height, then break the loop.
            if min_common_ancestor > latest_canon_height && sync_peers.len() >= threshold_to_request {
                break;
            }
        }

        println!(
            "Case 1 - min_common_ancestor: {min_common_ancestor}, latest_canon_height: {latest_canon_height}, sync_peers: {sync_peers:?}"
        );
        // If there is not enough peers with a minimum common ancestor above the latest canon height, then return early.
        if min_common_ancestor <= latest_canon_height || sync_peers.len() < threshold_to_request {
            return vec![];
        }

        // Initialize an RNG.
        let rng = &mut rand::thread_rng();

        // Compute the start height for the block request.
        let start_height = latest_canon_height + 1;
        // Compute the end height for the block request.
        let end_height = (min_common_ancestor + 1).min(start_height + DataBlocks::<N>::MAXIMUM_NUMBER_OF_BLOCKS as u32);

        self.construct_requests(start_height..end_height, sync_peers, candidate_locators, rng)

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

    /// Given the start height, end height, sync peers, this function returns a list of block requests.
    fn construct_requests<R: Rng + CryptoRng>(
        &self,
        heights: Range<u32>,
        sync_peers: IndexMap<SocketAddr, u32>,
        locators: IndexMap<SocketAddr, BlockLocators<N>>,
        rng: &mut R,
    ) -> Vec<(u32, SyncRequest<N>)> {
        let mut requests = Vec::with_capacity(heights.len());

        for height in heights {
            // Ensure the current height is not canonized or already requested.
            if self.check_block_request(height).is_err() {
                continue;
            }

            // Filter for the peer IPs that have this block.
            let peer_ips = sync_peers
                .iter()
                .filter(|(_, peer_height)| **peer_height >= height)
                .map(|(peer_ip, _)| *peer_ip)
                .collect::<Vec<_>>();

            // Construct the block request.
            let (hash, previous_hash, num_sync_ips) = construct_request(height, &peer_ips, &locators);
            // Pick the sync peers.
            let sync_ips = peer_ips.iter().copied().choose_multiple(rng, num_sync_ips);

            // Append the request.
            requests.push((height, (hash, previous_hash, sync_ips.into_iter().collect())));
        }

        requests
    }
}

/// If any peer is detected to be dishonest in this function, it will not set the hash or previous hash,
/// in order to allow the caller to determine what to do.
fn construct_request<N: Network>(
    height: u32,
    peer_ips: &[SocketAddr],
    locators: &IndexMap<SocketAddr, BlockLocators<N>>,
) -> (Option<N::BlockHash>, Option<N::BlockHash>, usize) {
    let mut hash = None;
    let mut hash_redundancy: usize = 0;
    let mut previous_hash = None;
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
                    Some(previous_hash) if previous_hash == candidate_previous_hash => (),
                    // Some peer is dishonest.
                    Some(_) => {
                        hash = None;
                        hash_redundancy = 0;
                        previous_hash = None;
                        is_honest = false;
                        break;
                    }
                    // Set the previous hash if it is not set.
                    None => previous_hash = Some(candidate_previous_hash),
                }
            }
        }
    }

    // Note that we intentionally do not just pick the peers that have the hash we have chosen,
    // to give stronger confidence that we are syncing during times when the network is consistent/stable.
    let num_sync_ips = {
        // Extra redundant peers - as the block hash was dishonest.
        if !is_honest {
            // TODO (howardwu): Consider performing an integrity check on peers (to disconnect).
            warn!("Detected dishonest peer(s) when preparing block request");
            // Choose up to the extra redundancy factor in sync peers.
            EXTRA_REDUNDANCY_FACTOR
        }
        // No redundant peers - as we have redundancy on the block hash.
        else if hash.is_some() && hash_redundancy >= REDUNDANCY_FACTOR {
            // Choose one sync peer.
            1
        }
        // Redundant peers - as we do not have redundancy on the block hash.
        else {
            // Choose up to the redundancy factor in sync peers.
            REDUNDANCY_FACTOR
        }
    };

    // Return the request.
    (hash, previous_hash, num_sync_ips)
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_node_messages::helpers::block_locators::test_helpers::sample_block_locators;
    use snarkvm::prelude::Field;

    use indexmap::indexset;
    use snarkos_node_messages::{CHECKPOINT_INTERVAL, NUM_RECENTS};
    use std::net::{IpAddr, Ipv4Addr};

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    /// Returns the local IP for the sync pool.
    fn sample_local_ip() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)
    }

    /// Returns the peer IP for the sync pool.
    fn sample_peer_ip(id: u16) -> SocketAddr {
        assert_ne!(id, 0, "The peer ID must not be 0 (reserved for local IP in testing)");
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), id)
    }

    /// Returns the sync pool, with the canonical map initialized to the given height.
    fn sample_sync_at_height(height: u32) -> Sync<CurrentNetwork> {
        let sync = Sync::<CurrentNetwork>::new(sample_local_ip());
        sync.insert_canon_locators(sample_block_locators(height)).unwrap();
        sync
    }

    /// Checks that the sync pool (starting at genesis) returns the correct requests.
    fn check_prepare_block_requests(sync: Sync<CurrentNetwork>, min_common_ancestor: u32, peers: IndexSet<SocketAddr>) {
        // Check test assumptions are met.
        assert_eq!(sync.latest_canon_height(), 0, "This test assumes the sync pool is at genesis");

        // Determine the number of peers within range of this sync pool.
        let num_peers_within_recent_range_of_canon = {
            // If no peers are within range, then set to 0.
            if min_common_ancestor >= NUM_RECENTS as u32 {
                0
            }
            // Otherwise, manually check the number of peers within range.
            else {
                peers.iter().filter(|peer_ip| sync.get_peer_height(peer_ip).unwrap() < NUM_RECENTS as u32).count()
            }
        };

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();

        // If there are no peers, then there should be no requests.
        if peers.is_empty() {
            assert!(requests.is_empty());
            return;
        }

        // Otherwise, there should be requests.
        let expected_num_requests = core::cmp::min(
            min_common_ancestor as usize,
            DataBlocks::<CurrentNetwork>::MAXIMUM_NUMBER_OF_BLOCKS as usize,
        );
        assert_eq!(requests.len(), expected_num_requests);

        for (idx, (height, (hash, previous_hash, sync_ips))) in requests.into_iter().enumerate() {
            assert_eq!(height, 1 + idx as u32);
            assert_eq!(hash, Some((Field::<CurrentNetwork>::from_u32(height)).into()));
            assert_eq!(previous_hash, Some((Field::<CurrentNetwork>::from_u32(height - 1)).into()));

            if num_peers_within_recent_range_of_canon >= REDUNDANCY_FACTOR {
                assert_eq!(sync_ips.len(), 1);
            } else {
                assert_eq!(sync_ips.len(), num_peers_within_recent_range_of_canon);
                assert_eq!(sync_ips, peers);
            }
        }
    }

    #[test]
    fn test_latest_canon_height() {
        for height in 0..100_002u32 {
            let sync = sample_sync_at_height(height);
            assert_eq!(sync.latest_canon_height(), height);
        }
    }

    #[test]
    fn test_get_canon_height() {
        for height in 0..100_002u32 {
            let sync = sample_sync_at_height(height);
            assert_eq!(sync.get_canon_height(&(Field::<CurrentNetwork>::from_u32(0)).into()), Some(0));
            assert_eq!(sync.get_canon_height(&(Field::<CurrentNetwork>::from_u32(height)).into()), Some(height));
        }
    }

    #[test]
    fn test_get_canon_hash() {
        for height in 0..100_002u32 {
            let sync = sample_sync_at_height(height);
            assert_eq!(sync.get_canon_hash(0), Some((Field::<CurrentNetwork>::from_u32(0)).into()));
            assert_eq!(sync.get_canon_hash(height), Some((Field::<CurrentNetwork>::from_u32(height)).into()));
        }
    }

    #[test]
    fn test_prepare_block_requests_with_no_peers() {
        let sync = sample_sync_at_height(0);

        // If there are no peers, then no requests should be prepared.
        check_prepare_block_requests(sync, 0, indexset![]);
    }

    #[test]
    fn test_prepare_block_requests_with_one_peer() {
        let sync = sample_sync_at_height(0);

        // Add a peer.
        sync.update_peer_locators(sample_peer_ip(1), sample_block_locators(10)).unwrap();

        // If a peer is ahead, then requests should be prepared (regardless of redundancy factor).
        check_prepare_block_requests(sync, 10, indexset![sample_peer_ip(1)]);
    }

    #[test]
    fn test_prepare_block_requests_with_two_peers() {
        let sync = sample_sync_at_height(0);

        // Add peer 1.
        sync.update_peer_locators(sample_peer_ip(1), sample_block_locators(10)).unwrap();
        // Add peer 2.
        sync.update_peer_locators(sample_peer_ip(2), sample_block_locators(10)).unwrap();

        // If both peers are ahead, then requests should be prepared (regardless of redundancy factor).
        check_prepare_block_requests(sync, 10, indexset![sample_peer_ip(1), sample_peer_ip(2)]);
    }

    #[test]
    fn test_prepare_block_requests_with_multiple_peers() {
        for num_peers in 2..100 {
            println!("Testing with {num_peers} peers");

            let sync = sample_sync_at_height(0);

            let mut peers = indexset![];

            for peer_id in 1..=num_peers {
                // Add a peer.
                sync.update_peer_locators(sample_peer_ip(peer_id), sample_block_locators(10)).unwrap();
                // Add the peer to the set of peers.
                peers.insert(sample_peer_ip(peer_id));
            }

            // If all peers are ahead, then requests should be prepared.
            check_prepare_block_requests(sync, 10, peers);
        }
    }

    #[test]
    fn test_update_peer_locators() {
        let sync = sample_sync_at_height(0);

        // Test 2 peers.
        let peer1_ip = sample_peer_ip(1);
        for peer1_height in 0..500u32 {
            sync.update_peer_locators(peer1_ip, sample_block_locators(peer1_height)).unwrap();
            assert_eq!(sync.get_peer_height(&peer1_ip), Some(peer1_height));

            let peer2_ip = sample_peer_ip(2);
            for peer2_height in 0..500u32 {
                println!("Testing peer 1 height at {peer1_height} and peer 2 height at {peer2_height}");

                sync.update_peer_locators(peer2_ip, sample_block_locators(peer2_height)).unwrap();
                assert_eq!(sync.get_peer_height(&peer2_ip), Some(peer2_height));

                // Compute the distance between the peers.
                let distance =
                    if peer1_height > peer2_height { peer1_height - peer2_height } else { peer2_height - peer1_height };

                // Check the common ancestor.
                if distance < NUM_RECENTS as u32 {
                    let expected_ancestor = core::cmp::min(peer1_height, peer2_height);
                    assert_eq!(sync.get_common_ancestor(peer1_ip, peer2_ip), Some(expected_ancestor));
                    assert_eq!(sync.get_common_ancestor(peer2_ip, peer1_ip), Some(expected_ancestor));
                } else {
                    let min_checkpoints =
                        core::cmp::min(peer1_height / CHECKPOINT_INTERVAL, peer2_height / CHECKPOINT_INTERVAL);
                    let expected_ancestor = min_checkpoints * CHECKPOINT_INTERVAL;
                    assert_eq!(sync.get_common_ancestor(peer1_ip, peer2_ip), Some(expected_ancestor));
                    assert_eq!(sync.get_common_ancestor(peer2_ip, peer1_ip), Some(expected_ancestor));
                }
            }
        }
    }

    #[test]
    fn test_remove_peer() {
        let sync = sample_sync_at_height(0);

        let peer_ip = sample_peer_ip(1);
        sync.update_peer_locators(peer_ip, sample_block_locators(100)).unwrap();
        assert_eq!(sync.get_peer_height(&peer_ip), Some(100));

        sync.remove_peer(&peer_ip);
        assert_eq!(sync.get_peer_height(&peer_ip), None);

        sync.update_peer_locators(peer_ip, sample_block_locators(200)).unwrap();
        assert_eq!(sync.get_peer_height(&peer_ip), Some(200));

        sync.remove_peer(&peer_ip);
        assert_eq!(sync.get_peer_height(&peer_ip), None);
    }

    // TODO: insert_block_req => insert_block_res => insert_block_req (same), ensure fails.
    // TODO: duplicate responses, ensure fails.
}
