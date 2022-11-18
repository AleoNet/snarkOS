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
use linked_hash_map::LinkedHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use time::OffsetDateTime;

/// The maximum number of items to store in the cache.
const MAX_CACHE_SIZE: usize = 256;

#[derive(Clone, Debug)]
pub struct Cache<N: Network> {
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: Arc<RwLock<LinkedHashMap<N::BlockHash, OffsetDateTime>>>,
    /// The map of solution commitments to their last seen timestamp.
    seen_inbound_solutions: Arc<RwLock<LinkedHashMap<PuzzleCommitment<N>, OffsetDateTime>>>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: Arc<RwLock<LinkedHashMap<N::TransactionID, OffsetDateTime>>>,
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
            seen_inbound_blocks: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_inbound_solutions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_inbound_transactions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_outbound_blocks: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_outbound_solutions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
            seen_outbound_transactions: Arc::new(RwLock::new(LinkedHashMap::with_capacity(MAX_CACHE_SIZE))),
        }
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
