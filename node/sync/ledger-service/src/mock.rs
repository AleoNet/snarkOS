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

use crate::LedgerService;
use parking_lot::Mutex;
use snarkvm::{
    ledger::block::Block,
    prelude::{ensure, Field, Network, Result},
};
use std::collections::BTreeMap;

/// A mock ledger service that always returns `false`.
#[derive(Debug)]
pub struct MockLedgerService<N: Network> {
    height_to_hash: Mutex<BTreeMap<u32, N::BlockHash>>,
}

impl<N: Network> MockLedgerService<N> {
    /// Creates a new mock ledger service.
    pub fn new_at_height(height: u32) -> Self {
        let mut height_to_hash = BTreeMap::new();
        for i in 0..=height {
            height_to_hash.insert(i, (Field::<N>::from_u32(i)).into());
        }
        Self { height_to_hash: Mutex::new(height_to_hash) }
    }
}

#[async_trait]
impl<N: Network> LedgerService<N> for MockLedgerService<N> {
    /// Returns the latest block height in the canonical ledger.
    fn latest_canon_height(&self) -> u32 {
        self.height_to_hash.lock().last_key_value().map(|(height, _)| *height).unwrap_or(0)
    }

    /// Returns `true` if the given block height exists in the canonical ledger.
    fn contains_canon_height(&self, height: u32) -> bool {
        self.height_to_hash.lock().contains_key(&height)
    }

    /// Returns the canonical block height for the given block hash, if it exists.
    fn get_canon_height(&self, hash: &N::BlockHash) -> Option<u32> {
        self.height_to_hash.lock().iter().find_map(|(height, h)| if h == hash { Some(*height) } else { None })
    }

    /// Returns the canonical block hash for the given block height, if it exists.
    fn get_canon_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.height_to_hash.lock().get(&height).cloned()
    }

    /// Checks the given block is valid next block.
    fn check_next_block(&self, _block: &Block<N>) -> Result<()> {
        Ok(())
    }

    /// Adds the given block as the next block in the ledger.
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        ensure!(
            block.height() == self.latest_canon_height() + 1,
            "Tried to advance to block {} from block {}",
            block.height(),
            self.latest_canon_height()
        );
        self.height_to_hash.lock().insert(block.height(), block.hash());
        Ok(())
    }
}
