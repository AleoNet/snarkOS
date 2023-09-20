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
use snarkvm::{
    ledger::{block::Block, store::ConsensusStorage, Ledger},
    prelude::{Network, Result},
};

use std::fmt;

/// A core ledger service.
pub struct CoreLedgerService<N: Network, C: ConsensusStorage<N>> {
    ledger: Ledger<N, C>,
}

impl<N: Network, C: ConsensusStorage<N>> CoreLedgerService<N, C> {
    /// Initializes a new core ledger service.
    pub fn new(ledger: Ledger<N, C>) -> Self {
        Self { ledger }
    }
}

impl<N: Network, C: ConsensusStorage<N>> fmt::Debug for CoreLedgerService<N, C> {
    /// Implements a custom `fmt::Debug` for `CoreLedgerService`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoreLedgerService").field("latest_canon_height", &self.latest_canon_height()).finish()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> LedgerService<N> for CoreLedgerService<N, C> {
    /// Returns the latest block height in the canonical ledger.
    fn latest_canon_height(&self) -> u32 {
        self.ledger.latest_height()
    }

    /// Returns `true` if the given block height exists in the canonical ledger.
    fn contains_canon_height(&self, height: u32) -> bool {
        self.ledger.contains_block_height(height).unwrap_or(false)
    }

    /// Returns the canonical block height for the given block hash, if it exists.
    fn get_canon_height(&self, hash: &N::BlockHash) -> Option<u32> {
        self.ledger.get_height(hash).ok()
    }

    /// Returns the canonical block hash for the given block height, if it exists.
    fn get_canon_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.ledger.get_hash(height).ok()
    }

    /// Checks the given block is valid next block.
    fn check_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.check_next_block(block)
    }

    /// Adds the given block as the next block in the ledger.
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.advance_to_next_block(block)
    }
}
