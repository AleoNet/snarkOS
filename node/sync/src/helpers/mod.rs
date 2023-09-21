// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::locators::{BlockLocators, CHECKPOINT_INTERVAL, NUM_RECENT_BLOCKS};
use snarkvm::{
    ledger::{store::ConsensusStorage, Ledger},
    prelude::Network,
};

use anyhow::Result;
use core::hash::Hash;
use indexmap::{IndexMap, IndexSet};
use std::net::SocketAddr;

/// A tuple of the block hash (optional), previous block hash (optional), and sync IPs.
pub type SyncRequest<N> = (Option<<N as Network>::BlockHash>, Option<<N as Network>::BlockHash>, IndexSet<SocketAddr>);

#[derive(Copy, Clone, Debug)]
pub(crate) struct PeerPair(pub SocketAddr, pub SocketAddr);

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

/// Returns the block locators for the given ledger.
pub fn get_block_locators<N: Network, C: ConsensusStorage<N>>(ledger: &Ledger<N, C>) -> Result<BlockLocators<N>> {
    // Retrieve the latest height.
    let latest_height = ledger.latest_height();

    // Initialize the recents map.
    let mut recents = IndexMap::with_capacity(NUM_RECENT_BLOCKS);

    // Retrieve the recent block hashes.
    for height in latest_height.saturating_sub((NUM_RECENT_BLOCKS - 1) as u32)..=latest_height {
        recents.insert(height, ledger.get_hash(height)?);
    }

    // Initialize the checkpoints map.
    let mut checkpoints = IndexMap::with_capacity((latest_height / CHECKPOINT_INTERVAL + 1).try_into()?);

    // Retrieve the checkpoint block hashes.
    for height in (0..=latest_height).step_by(CHECKPOINT_INTERVAL as usize) {
        checkpoints.insert(height, ledger.get_hash(height)?);
    }

    // Construct the block locators.
    BlockLocators::new(recents, checkpoints)
}
