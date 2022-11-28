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

use snarkvm::prelude::{Network, PuzzleCommitment};

use core::hash::Hash;
use indexmap::IndexMap;
use linked_hash_map::LinkedHashMap;
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicU16, Ordering::SeqCst},
        Arc,
    },
};
use time::{Duration, OffsetDateTime};

/// The maximum number of items to store in the cache.
const MAX_CACHE_SIZE: usize = 256;

#[derive(Clone, Debug)]
pub struct Cache<N: Network> {
    /// The map of peer connections to their recent timestamps.
    seen_inbound_connections: Arc<RwLock<IndexMap<IpAddr, VecDeque<OffsetDateTime>>>>,
    /// The map of peer IPs to their recent timestamps.
    seen_inbound_messages: Arc<RwLock<IndexMap<SocketAddr, VecDeque<OffsetDateTime>>>>,
    /// The map of peer IPs to their recent timestamps.
    seen_inbound_puzzle_requests: Arc<RwLock<IndexMap<SocketAddr, VecDeque<OffsetDateTime>>>>,
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: Arc<RwLock<LinkedHashMap<N::BlockHash, OffsetDateTime>>>,
    /// The map of solution commitments to their last seen timestamp.
    seen_inbound_solutions: Arc<RwLock<LinkedHashMap<PuzzleCommitment<N>, OffsetDateTime>>>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: Arc<RwLock<LinkedHashMap<N::TransactionID, OffsetDateTime>>>,
    /// The map of peer IPs to the number of puzzle requests.
    seen_outbound_puzzle_requests: Arc<RwLock<IndexMap<SocketAddr, Arc<AtomicU16>>>>,
    /// The map of block hashes to their last seen timestamp.
    seen_outbound_blocks: Arc<RwLock<LinkedHashMap<N::BlockHash, OffsetDateTime>>>,
    /// The map of solution commitments to their last seen timestamp.
    seen_outbound_solutions: Arc<RwLock<LinkedHashMap<PuzzleCommitment<N>, OffsetDateTime>>>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: Arc<RwLock<LinkedHashMap<N::TransactionID, OffsetDateTime>>>,
}

impl<N: Network> Default for Cache<N> {
    /// Initializes a new instance of the cache.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Cache<N> {
    /// Initializes a new instance of the cache.
    pub fn new() -> Self {
        Self {
            seen_inbound_connections: Default::default(),
            seen_inbound_messages: Default::default(),
            seen_inbound_puzzle_requests: Default::default(),
            seen_inbound_blocks: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_inbound_solutions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_inbound_transactions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_outbound_puzzle_requests: Default::default(),
            seen_outbound_blocks: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_outbound_solutions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_outbound_transactions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
        }
    }

    /// Inserts a new timestamp for the given peer connection, returning the number of recent connection requests.
    pub fn insert_inbound_connection(&self, peer_ip: IpAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_inbound_connections, peer_ip, interval_in_secs)
    }

    /// Inserts a new timestamp for the given peer message, returning the number of recent messages.
    pub fn insert_inbound_message(&self, peer_ip: SocketAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_inbound_messages, peer_ip, interval_in_secs)
    }

    /// Inserts a new timestamp for the given peer IP, returning the number of recent requests.
    pub fn insert_inbound_puzzle_request(&self, peer_ip: SocketAddr) -> usize {
        Self::retain_and_insert(&self.seen_inbound_puzzle_requests, peer_ip, 60)
    }

    /// Returns `true` if the cache contains a puzzle request from the given peer.
    pub fn contains_outbound_puzzle_request(&self, peer_ip: &SocketAddr) -> bool {
        self.seen_outbound_puzzle_requests.read().contains_key(peer_ip)
    }

    /// Increment the peer IP's number of puzzle requests, returning the updated number of puzzle requests.
    pub fn increment_outbound_puzzle_requests(&self, peer_ip: SocketAddr) -> u16 {
        Self::increment_counter(&self.seen_outbound_puzzle_requests, peer_ip)
    }

    /// Decrement the peer IP's number of puzzle requests, returning the updated number of puzzle requests.
    pub fn decrement_outbound_puzzle_requests(&self, peer_ip: SocketAddr) -> u16 {
        Self::decrement_counter(&self.seen_outbound_puzzle_requests, peer_ip)
    }

    /// Inserts a block hash into the cache, returning the previously seen timestamp if it existed.
    pub fn insert_inbound_block(&self, hash: N::BlockHash) -> Option<OffsetDateTime> {
        Self::refresh_and_insert(&self.seen_inbound_blocks, hash)
    }

    /// Inserts a solution commitment into the cache, returning the previously seen timestamp if it existed.
    pub fn insert_inbound_solution(&self, solution: PuzzleCommitment<N>) -> Option<OffsetDateTime> {
        Self::refresh_and_insert(&self.seen_inbound_solutions, solution)
    }

    /// Inserts a transaction ID into the cache, returning the previously seen timestamp if it existed.
    pub fn insert_inbound_transaction(&self, transaction: N::TransactionID) -> Option<OffsetDateTime> {
        Self::refresh_and_insert(&self.seen_inbound_transactions, transaction)
    }

    /// Inserts a block hash into the cache, returning the previously seen timestamp if it existed.
    pub fn insert_outbound_block(&self, hash: N::BlockHash) -> Option<OffsetDateTime> {
        Self::refresh_and_insert(&self.seen_outbound_blocks, hash)
    }

    /// Inserts a solution commitment into the cache, returning the previously seen timestamp if it existed.
    pub fn insert_outbound_solution(&self, solution: PuzzleCommitment<N>) -> Option<OffsetDateTime> {
        Self::refresh_and_insert(&self.seen_outbound_solutions, solution)
    }

    /// Inserts a transaction ID into the cache, returning the previously seen timestamp if it existed.
    pub fn insert_outbound_transaction(&self, transaction: N::TransactionID) -> Option<OffsetDateTime> {
        Self::refresh_and_insert(&self.seen_outbound_transactions, transaction)
    }
}

impl<N: Network> Cache<N> {
    /// Insert a new timestamp for the given key, returning the number of recent entries.
    fn retain_and_insert<K: Eq + Hash + Clone>(
        map: &Arc<RwLock<IndexMap<K, VecDeque<OffsetDateTime>>>>,
        key: K,
        interval_in_secs: i64,
    ) -> usize {
        let mut map_write = map.write();
        // Load the entry for the key.
        let timestamps = map_write.entry(key).or_default();
        // Fetch the current timestamp.
        let now = OffsetDateTime::now_utc();
        // Insert the new timestamp.
        timestamps.push_back(now);
        // Retain only the timestamps that are within the recent interval.
        while timestamps.iter().next().map_or(false, |t| now - *t > Duration::seconds(interval_in_secs)) {
            timestamps.pop_front();
        }
        // Return the frequency of recent requests.
        timestamps.len()
    }

    /// Increments the key's counter in the map, returning the updated counter.
    fn increment_counter<K: Hash + Eq>(map: &Arc<RwLock<IndexMap<K, Arc<AtomicU16>>>>, key: K) -> u16 {
        // Load the entry for the key, and increment the counter.
        let previous_entry = map.write().entry(key).or_default().fetch_add(1, SeqCst);
        // Return the updated counter.
        previous_entry.saturating_add(1)
    }

    /// Decrements the key's counter in the map, returning the updated counter.
    fn decrement_counter<K: Hash + Eq>(map: &Arc<RwLock<IndexMap<K, Arc<AtomicU16>>>>, key: K) -> u16 {
        let mut map_write = map.write();
        // Load the entry for the key.
        let entry = map_write.entry(key).or_default();
        // Conditionally decrement the counter.
        match entry.load(SeqCst) > 0 {
            true => entry.fetch_sub(1, SeqCst).saturating_sub(1),
            false => 0,
        }
    }

    /// Updates the map by enforcing the maximum cache size.
    fn refresh<K: Eq + Hash, V>(map: &RwLock<LinkedHashMap<K, V>>) {
        let mut map_write = map.write();
        while map_write.len() >= MAX_CACHE_SIZE {
            map_write.pop_front();
        }
    }

    /// Updates the map by enforcing the maximum cache size, and inserts the given key.
    /// Returns the previously seen timestamp if it existed.
    fn refresh_and_insert<K: Eq + Hash>(
        map: &RwLock<LinkedHashMap<K, OffsetDateTime>>,
        key: K,
    ) -> Option<OffsetDateTime> {
        Self::refresh(map);
        map.write().insert(key, OffsetDateTime::now_utc())
    }
}
