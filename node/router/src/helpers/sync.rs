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

use anyhow::{bail, ensure, Result};
use colored::Colorize;
use core::hash::Hash;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use parking_lot::RwLock;
use rand::{prelude::IteratorRandom, CryptoRng, Rng};
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Instant};

pub const REDUNDANCY_FACTOR: usize = 3;
pub const EXTRA_REDUNDANCY_FACTOR: usize = REDUNDANCY_FACTOR * 2;
pub const NUM_SYNC_CANDIDATE_PEERS: usize = REDUNDANCY_FACTOR * 5;

pub const BLOCK_REQUEST_TIMEOUT_IN_SECS: u64 = 15; // 15 seconds
pub const MAX_BLOCK_REQUESTS: usize = 50; // 50 requests
pub const MAX_BLOCK_REQUEST_TIMEOUTS: usize = 5; // 5 timeouts

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

/// A struct that keeps track of the current sync state.
///
/// # State
/// - When a request is inserted, the `requests` map and `request_timestamps` map insert an entry for the request height.
/// - When a response is inserted, the `requests` map inserts the entry for the request height.
/// - When a request is completed, the `requests` map still has the entry, but its `sync_ips` is empty;
/// - the `request_timestamps` map remains unchanged.
/// - When a response is removed/completed, the `requests` map and `request_timestamps` map also remove the entry for the request height.
/// - When a request is timed out, the `requests`, `request_timestamps`, and `responses` map remove the entry for the request height;
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
    /// The map of block height to the timestamp of the last time the block was requested.
    /// This map is used to determine which requests to remove if they have been pending for too long.
    request_timestamps: Arc<RwLock<BTreeMap<u32, Instant>>>,
    /// The map of (timed out) peer IPs to their request timestamps.
    /// This map is used to determine which peers to remove if they have timed out too many times.
    request_timeouts: Arc<RwLock<IndexMap<SocketAddr, Vec<Instant>>>>,
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
            request_timestamps: Default::default(),
            request_timeouts: Default::default(),
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

    /// Returns the block request for the given height, if it exists.
    pub fn get_block_request(&self, height: u32) -> Option<SyncRequest<N>> {
        self.requests.read().get(&height).cloned()
    }

    /// Returns the timestamp of the last time the block was requested, if it exists.
    pub fn get_block_request_timestamp(&self, height: u32) -> Option<Instant> {
        self.request_timestamps.read().get(&height).copied()
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

    /// Returns the sync peers with their latest heights, and their minimum common ancestor, if the node can sync.
    /// This function returns peers that are consistent with each other, and have a block height
    /// that is greater than the canon height of this node.
    pub fn find_sync_peers(&self) -> Option<(IndexMap<SocketAddr, u32>, u32)> {
        if let Some((sync_peers, min_common_ancestor)) = self.find_sync_peers_inner() {
            // Map the locators into the latest height.
            let sync_peers =
                sync_peers.into_iter().map(|(ip, locators)| (ip, locators.latest_locator_height())).collect();
            // Return the sync peers and their minimum common ancestor.
            Some((sync_peers, min_common_ancestor))
        } else {
            None
        }
    }

    /// Returns a list of block requests, if the node needs to sync.
    pub fn prepare_block_requests(&self) -> Vec<(u32, SyncRequest<N>)> {
        // Remove timed out block requests.
        self.remove_timed_out_block_requests();
        // Prepare the block requests.
        if let Some((sync_peers, min_common_ancestor)) = self.find_sync_peers_inner() {
            // Return the list of block requests.
            self.construct_requests(sync_peers, min_common_ancestor, &mut rand::thread_rng())
        } else {
            // Return an empty list of block requests.
            Vec::new()
        }
    }

    /// Inserts a block request for the given height.
    pub fn insert_block_request(&self, height: u32, (hash, previous_hash, sync_ips): SyncRequest<N>) -> Result<()> {
        // Ensure the block request does not already exist.
        self.check_block_request(height)?;
        // Ensure the sync IPs are not empty.
        ensure!(!sync_ips.is_empty(), "Cannot insert a block request with no sync IPs");
        // Insert the block request.
        self.requests.write().insert(height, (hash, previous_hash, sync_ips));
        // Insert the request timestamp.
        self.request_timestamps.write().insert(height, Instant::now());
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
            self.remove_block_requests_to_peer(&peer_ip);
            return Err(error);
        }

        // Remove the peer IP from the request entry.
        if let Some((_, _, sync_ips)) = self.requests.write().get_mut(&height) {
            sync_ips.remove(&peer_ip);
        }

        // Acquire the write lock on the responses map.
        let mut responses = self.responses.write();
        // Insert the candidate block into the responses map.
        if let Some(existing_block) = responses.insert(height, block.clone()) {
            // If the candidate block was already present, ensure it is the same block.
            if block != existing_block {
                // Remove the candidate block.
                responses.remove(&height);
                // Remove all block requests to the peer.
                self.remove_block_requests_to_peer(&peer_ip);
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

    /// Removes the peer from the sync pool, if they exist.
    pub fn remove_peer(&self, peer_ip: &SocketAddr) {
        // Remove the locators entry for the given peer IP.
        self.locators.write().remove(peer_ip);
        // Remove all block requests to the peer.
        self.remove_block_requests_to_peer(peer_ip);
        // Remove the timeouts for the peer.
        self.request_timeouts.write().remove(peer_ip);
    }

    /// Removes the block request for the given peer IP, if it exists.
    pub fn remove_block_request_to_peer(&self, peer_ip: &SocketAddr, height: u32) {
        let mut can_revoke = self.responses.read().get(&height).is_none();

        // Remove the peer IP from the request entry. If the request entry is now empty,
        // and the response entry for this height is also empty, then remove the request entry altogether.
        if let Some((_, _, sync_ips)) = self.requests.write().get_mut(&height) {
            sync_ips.remove(peer_ip);
            can_revoke &= sync_ips.is_empty();
        }

        if can_revoke {
            self.requests.write().remove(&height);
            self.request_timestamps.write().remove(&height);
        }
    }

    /// Removes all block requests for the given peer IP.
    pub fn remove_block_requests_to_peer(&self, peer_ip: &SocketAddr) {
        // Acquire the write lock on the requests map.
        let mut requests = self.requests.write();
        // Acquire the read lock on the responses map.
        let responses = self.responses.read();

        // Remove the peer IP from the requests map. If any request entry is now empty,
        // and its corresponding response entry is also empty, then remove that request entry altogether.
        requests.retain(|height, (_, _, peer_ips)| {
            peer_ips.remove(peer_ip);

            let retain = !peer_ips.is_empty() || responses.get(height).is_some();
            if !retain {
                self.request_timestamps.write().remove(height);
            }
            retain
        });
    }

    /// Removes the entire block request for the given height, if it exists.
    pub fn remove_block_request(&self, height: u32) {
        // Remove the request entry for the given height.
        self.requests.write().remove(&height);
        // Remove the response entry for the given height.
        self.responses.write().remove(&height);
        // Remove the request timestamp entry for the given height.
        self.request_timestamps.write().remove(&height);
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
        // Ensure the block height is not already requested.
        if self.request_timestamps.read().contains_key(&height) {
            bail!("Failed to add block request, as block {height} exists in the timestamps map");
        }
        Ok(())
    }

    /// Checks the given block (response) from a peer against the expected block hash and previous block hash.
    fn check_block_response(&self, peer_ip: &SocketAddr, block: &Block<N>) -> Result<()> {
        // Retrieve the block height.
        let height = block.height();

        // Retrieve the request entry for the candidate block.
        if let Some((expected_hash, expected_previous_hash, sync_ips)) = self.requests.read().get(&height) {
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
            if !sync_ips.contains(peer_ip) {
                bail!("The sync pool did not request block {height} from '{peer_ip}'")
            }
            Ok(())
        } else {
            bail!("The sync pool did not request block {height}")
        }
    }

    /// Removes block requests that have timed out. This also removes the corresponding block responses,
    /// and adds the timed out sync IPs to a map for tracking. Returns the number of timed out block requests.
    fn remove_timed_out_block_requests(&self) -> usize {
        // Acquire the write lock on the requests map.
        let mut requests = self.requests.write();
        // Acquire the write lock on the responses map.
        let mut responses = self.responses.write();
        // Acquire the write lock on the request timestamps map.
        let mut request_timestamps = self.request_timestamps.write();

        // Retrieve the current time.
        let now = Instant::now();

        // Track each unique peer IP that has timed out.
        let mut timeout_ips = IndexSet::new();
        // Track the number of timed out block requests.
        let mut num_timed_out_block_requests = 0;

        // Remove timed out block requests.
        request_timestamps.retain(|height, timestamp| {
            // Determine if the duration since the request timestamp has exceeded the request timeout.
            let is_time_passed = now.duration_since(*timestamp).as_secs() > BLOCK_REQUEST_TIMEOUT_IN_SECS;
            // Determine if the request is incomplete.
            let is_request_incomplete =
                !requests.get(height).map(|(_, _, peer_ips)| peer_ips.is_empty()).unwrap_or(false);
            // Determine if the request has timed out.
            let is_timeout = is_time_passed && is_request_incomplete;

            // If the request has timed out, then remove it.
            if is_timeout {
                // Remove the request entry for the given height.
                if let Some((_, _, sync_ips)) = requests.remove(height) {
                    // Add each sync IP to the timeout IPs.
                    timeout_ips.extend(sync_ips);
                }
                // Remove the response entry for the given height.
                responses.remove(height);
                // Increment the number of timed out block requests.
                num_timed_out_block_requests += 1;
            }
            // Retain if this is not a timeout.
            !is_timeout
        });

        // If there are timeout IPs, then add them to the request timeouts map.
        if !timeout_ips.is_empty() {
            // Acquire the write lock on the request timeouts map.
            let mut request_timeouts = self.request_timeouts.write();
            // Add each timeout IP to the request timeouts map.
            for timeout_ip in timeout_ips {
                request_timeouts.entry(timeout_ip).or_default().push(now);
            }
        }

        num_timed_out_block_requests
    }

    /// Returns the sync peers and their minimum common ancestor, if the node needs to sync.
    fn find_sync_peers_inner(&self) -> Option<(IndexMap<SocketAddr, BlockLocators<N>>, u32)> {
        // Retrieve the latest canon height.
        let latest_canon_height = self.latest_canon_height();

        // Compute the timeout frequency of each peer.
        let timeouts = self
            .request_timeouts
            .read()
            .iter()
            .map(|(peer_ip, timestamps)| (*peer_ip, timestamps.len()))
            .collect::<IndexMap<_, _>>();

        // Pick a set of peers above the latest canon height, and include their locators.
        let candidate_locators: IndexMap<_, _> = self
            .locators
            .read()
            .iter()
            .filter(|(_, locators)| locators.latest_locator_height() > latest_canon_height)
            .filter(|(ip, _)| timeouts.get(*ip).map(|count| *count < MAX_BLOCK_REQUEST_TIMEOUTS).unwrap_or(true))
            .sorted_by(|(_, a), (_, b)| b.latest_locator_height().cmp(&a.latest_locator_height()))
            .take(NUM_SYNC_CANDIDATE_PEERS)
            .map(|(peer_ip, locators)| (*peer_ip, locators.clone()))
            .collect();

        // Case 0: If there are no candidate peers, return `None`.
        if candidate_locators.is_empty() {
            return None;
        }

        // TODO (howardwu): Change this to the highest cumulative weight for Phase 3.
        // Case 1: If all of the candidate peers share a common ancestor below the latest canon height,
        // then pick the peer with the highest height, and find peers (up to extra redundancy) with
        // a common ancestor above the block request range. Set the end height to their common ancestor.

        // Determine the threshold number of peers to sync from.
        let threshold_to_request = core::cmp::min(candidate_locators.len(), REDUNDANCY_FACTOR);

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
            sync_peers.insert(*peer_ip, peer_locators.clone());

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
                            sync_peers.insert(*other_ip, other_locators.clone());
                        }
                    }
                }
            }

            // If we have enough sync peers above the latest canon height, then break the loop.
            if min_common_ancestor > latest_canon_height && sync_peers.len() >= threshold_to_request {
                break;
            }
        }

        // If there is not enough peers with a minimum common ancestor above the latest canon height, then return early.
        if min_common_ancestor <= latest_canon_height || sync_peers.len() < threshold_to_request {
            return None;
        }

        Some((sync_peers, min_common_ancestor))
    }

    /// Given the sync peers and their minimum common ancestor, return a list of block requests.
    fn construct_requests<R: Rng + CryptoRng>(
        &self,
        sync_peers: IndexMap<SocketAddr, BlockLocators<N>>,
        min_common_ancestor: u32,
        rng: &mut R,
    ) -> Vec<(u32, SyncRequest<N>)> {
        // Retrieve the latest canon height.
        let latest_canon_height = self.latest_canon_height();

        // If the minimum common ancestor is at or below the latest canon height, then return early.
        if min_common_ancestor <= latest_canon_height {
            return vec![];
        }

        // Compute the start height for the block request.
        let start_height = latest_canon_height + 1;
        // Compute the end height for the block request.
        let end_height = (min_common_ancestor + 1).min(start_height + MAX_BLOCK_REQUESTS as u32);

        let mut requests = Vec::with_capacity((start_height..end_height).len());

        for height in start_height..end_height {
            // Ensure the current height is not canonized or already requested.
            if self.check_block_request(height).is_err() {
                continue;
            }

            // Construct the block request.
            let (hash, previous_hash, num_sync_ips, is_honest) = construct_request(height, &sync_peers);

            // Handle the dishonest case.
            if !is_honest {
                // TODO (howardwu): Consider performing an integrity check on peers (to disconnect).
                warn!("Detected dishonest peer(s) when preparing block request");
                // If there are not enough peers in the dishonest case, then return early.
                if sync_peers.len() < num_sync_ips {
                    break;
                }
            }

            // Pick the sync peers.
            let sync_ips = sync_peers.keys().copied().choose_multiple(rng, num_sync_ips);

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
    sync_peers: &IndexMap<SocketAddr, BlockLocators<N>>,
) -> (Option<N::BlockHash>, Option<N::BlockHash>, usize, bool) {
    let mut hash = None;
    let mut hash_redundancy: usize = 0;
    let mut previous_hash = None;
    let mut is_honest = true;

    for peer_locators in sync_peers.values() {
        if let Some(candidate_hash) = peer_locators.get_hash(height) {
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
        if let Some(candidate_previous_hash) = peer_locators.get_hash(height.saturating_sub(1)) {
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

    // Note that we intentionally do not just pick the peers that have the hash we have chosen,
    // to give stronger confidence that we are syncing during times when the network is consistent/stable.
    let num_sync_ips = {
        // Extra redundant peers - as the block hash was dishonest.
        if !is_honest {
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

    (hash, previous_hash, num_sync_ips, is_honest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_node_messages::helpers::block_locators::test_helpers::{
        sample_block_locators,
        sample_block_locators_with_fork,
    };
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
        let expected_num_requests = core::cmp::min(min_common_ancestor as usize, MAX_BLOCK_REQUESTS);
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
    fn test_prepare_block_requests() {
        for num_peers in 0..111 {
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
    fn test_prepare_block_requests_with_leading_fork_at_11() {
        let sync = sample_sync_at_height(0);

        // Intuitively, peer 1's fork is above peer 2 and peer 3's height.
        // So from peer 2 and peer 3's perspective, they don't even realize that peer 1 is on a fork.
        // Thus, you can sync up to block 10 from any of the 3 peers.

        // When there are NUM_REDUNDANCY peers ahead, and 1 peer is on a leading fork at 11,
        // then the sync pool should request blocks 1..=10 from the NUM_REDUNDANCY peers.
        // This is safe because the leading fork is at 11, and the sync pool is at 0,
        // so all candidate peers are at least 10 blocks ahead of the sync pool.

        // Add a peer (fork).
        let peer_1 = sample_peer_ip(1);
        sync.update_peer_locators(peer_1, sample_block_locators_with_fork(20, 11)).unwrap();

        // Add a peer.
        let peer_2 = sample_peer_ip(2);
        sync.update_peer_locators(peer_2, sample_block_locators(10)).unwrap();

        // Add a peer.
        let peer_3 = sample_peer_ip(3);
        sync.update_peer_locators(peer_3, sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 10);

        // Check the requests.
        for (idx, (height, (hash, previous_hash, sync_ips))) in requests.into_iter().enumerate() {
            assert_eq!(height, 1 + idx as u32);
            assert_eq!(hash, Some((Field::<CurrentNetwork>::from_u32(height)).into()));
            assert_eq!(previous_hash, Some((Field::<CurrentNetwork>::from_u32(height - 1)).into()));
            assert_eq!(sync_ips.len(), 1); // Only 1 needed since we have redundancy factor on this (recent locator) hash.
        }
    }

    #[test]
    fn test_prepare_block_requests_with_leading_fork_at_10() {
        let sync = sample_sync_at_height(0);

        // Intuitively, peer 1's fork is at peer 2 and peer 3's height.
        // So from peer 2 and peer 3's perspective, they recognize that peer 1 has forked.
        // Thus, you don't have NUM_REDUNDANCY peers to sync to block 10.
        //
        // Now, while you could in theory sync up to block 9 from any of the 3 peers,
        // we choose not to do this as either side is likely to disconnect from us,
        // and we would rather wait for enough redundant peers before syncing.

        // When there are NUM_REDUNDANCY peers ahead, and 1 peer is on a leading fork at 10,
        // then the sync pool should not request blocks as 1 peer conflicts with the other NUM_REDUNDANCY-1 peers.
        // We choose to sync with a cohort of peers that are *consistent* with each other,
        // and prioritize from descending heights (so the highest peer gets priority).

        // Add a peer (fork).
        let peer_1 = sample_peer_ip(1);
        sync.update_peer_locators(peer_1, sample_block_locators_with_fork(20, 10)).unwrap();

        // Add a peer.
        let peer_2 = sample_peer_ip(2);
        sync.update_peer_locators(peer_2, sample_block_locators(10)).unwrap();

        // Add a peer.
        let peer_3 = sample_peer_ip(3);
        sync.update_peer_locators(peer_3, sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 0);

        // When there are NUM_REDUNDANCY+1 peers ahead, and 1 is on a fork, then there should be block requests.

        // Add a peer.
        let peer_4 = sample_peer_ip(4);
        sync.update_peer_locators(peer_4, sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 10);

        // Check the requests.
        for (idx, (height, (hash, previous_hash, sync_ips))) in requests.into_iter().enumerate() {
            assert_eq!(height, 1 + idx as u32);
            assert_eq!(hash, Some((Field::<CurrentNetwork>::from_u32(height)).into()));
            assert_eq!(previous_hash, Some((Field::<CurrentNetwork>::from_u32(height - 1)).into()));
            assert_eq!(sync_ips.len(), 1); // Only 1 needed since we have redundancy factor on this (recent locator) hash.
            assert_ne!(sync_ips[0], peer_1); // It should never be the forked peer.
        }
    }

    #[test]
    fn test_prepare_block_requests_with_trailing_fork_at_9() {
        let sync = sample_sync_at_height(0);

        // Peer 1 and 2 diverge from peer 3 at block 10. We only sync when there are NUM_REDUNDANCY peers
        // who are *consistent* with each other. So if you add a 4th peer that is consistent with peer 1 and 2,
        // then you should be able to sync up to block 10, thereby biasing away from peer 3.

        // Add a peer (fork).
        let peer_1 = sample_peer_ip(1);
        sync.update_peer_locators(peer_1, sample_block_locators(10)).unwrap();

        // Add a peer.
        let peer_2 = sample_peer_ip(2);
        sync.update_peer_locators(peer_2, sample_block_locators(10)).unwrap();

        // Add a peer.
        let peer_3 = sample_peer_ip(3);
        sync.update_peer_locators(peer_3, sample_block_locators_with_fork(20, 10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 0);

        // When there are NUM_REDUNDANCY+1 peers ahead, and peer 3 is on a fork, then there should be block requests.

        // Add a peer.
        let peer_4 = sample_peer_ip(4);
        sync.update_peer_locators(peer_4, sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 10);

        // Check the requests.
        for (idx, (height, (hash, previous_hash, sync_ips))) in requests.into_iter().enumerate() {
            assert_eq!(height, 1 + idx as u32);
            assert_eq!(hash, Some((Field::<CurrentNetwork>::from_u32(height)).into()));
            assert_eq!(previous_hash, Some((Field::<CurrentNetwork>::from_u32(height - 1)).into()));
            assert_eq!(sync_ips.len(), 1); // Only 1 needed since we have redundancy factor on this (recent locator) hash.
            assert_ne!(sync_ips[0], peer_3); // It should never be the forked peer.
        }
    }

    #[test]
    fn test_insert_block_requests() {
        let sync = sample_sync_at_height(0);

        // Add a peer.
        sync.update_peer_locators(sample_peer_ip(1), sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 10);

        for (height, (hash, previous_hash, sync_ips)) in requests.clone() {
            // Insert the block request.
            sync.insert_block_request(height, (hash, previous_hash, sync_ips.clone())).unwrap();
            // Check that the block requests were inserted.
            assert_eq!(sync.get_block_request(height), Some((hash, previous_hash, sync_ips)));
            assert!(sync.get_block_request_timestamp(height).is_some());
        }

        for (height, (hash, previous_hash, sync_ips)) in requests.clone() {
            // Check that the block requests are still inserted.
            assert_eq!(sync.get_block_request(height), Some((hash, previous_hash, sync_ips)));
            assert!(sync.get_block_request_timestamp(height).is_some());
        }

        for (height, (hash, previous_hash, sync_ips)) in requests {
            // Ensure that the block requests cannot be inserted twice.
            sync.insert_block_request(height, (hash, previous_hash, sync_ips.clone())).unwrap_err();
            // Check that the block requests are still inserted.
            assert_eq!(sync.get_block_request(height), Some((hash, previous_hash, sync_ips)));
            assert!(sync.get_block_request_timestamp(height).is_some());
        }
    }

    #[test]
    fn test_insert_block_requests_fails() {
        let sync = sample_sync_at_height(9);

        // Add a peer.
        sync.update_peer_locators(sample_peer_ip(1), sample_block_locators(10)).unwrap();

        // Inserting a block height that is already canonized should fail.
        sync.insert_block_request(9, (None, None, indexset![sample_peer_ip(1)])).unwrap_err();
        // Inserting a block height that is not canonized should succeed.
        sync.insert_block_request(10, (None, None, indexset![sample_peer_ip(1)])).unwrap();
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

    #[test]
    fn test_locators_insert_remove_insert() {
        let sync = sample_sync_at_height(0);

        let peer_ip = sample_peer_ip(1);
        sync.update_peer_locators(peer_ip, sample_block_locators(100)).unwrap();
        assert_eq!(sync.get_peer_height(&peer_ip), Some(100));

        sync.remove_peer(&peer_ip);
        assert_eq!(sync.get_peer_height(&peer_ip), None);

        sync.update_peer_locators(peer_ip, sample_block_locators(200)).unwrap();
        assert_eq!(sync.get_peer_height(&peer_ip), Some(200));
    }

    #[test]
    fn test_requests_insert_remove_insert() {
        let sync = sample_sync_at_height(0);

        // Add a peer.
        let peer_ip = sample_peer_ip(1);
        sync.update_peer_locators(peer_ip, sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 10);

        for (height, (hash, previous_hash, sync_ips)) in requests.clone() {
            // Insert the block request.
            sync.insert_block_request(height, (hash, previous_hash, sync_ips.clone())).unwrap();
            // Check that the block requests were inserted.
            assert_eq!(sync.get_block_request(height), Some((hash, previous_hash, sync_ips)));
            assert!(sync.get_block_request_timestamp(height).is_some());
        }

        // Remove the peer.
        sync.remove_peer(&peer_ip);

        for (height, _) in requests {
            // Check that the block requests were removed.
            assert_eq!(sync.get_block_request(height), None);
            assert!(sync.get_block_request_timestamp(height).is_none());
        }

        // As there is no peer, it should not be possible to prepare block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 0);

        // Add the peer again.
        sync.update_peer_locators(peer_ip, sample_block_locators(10)).unwrap();

        // Prepare the block requests.
        let requests = sync.prepare_block_requests();
        assert_eq!(requests.len(), 10);

        for (height, (hash, previous_hash, sync_ips)) in requests {
            // Insert the block request.
            sync.insert_block_request(height, (hash, previous_hash, sync_ips.clone())).unwrap();
            // Check that the block requests were inserted.
            assert_eq!(sync.get_block_request(height), Some((hash, previous_hash, sync_ips)));
            assert!(sync.get_block_request_timestamp(height).is_some());
        }
    }

    // TODO: duplicate responses, ensure fails.
}
