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

use snarkvm::{
    ledger::block::Block,
    prelude::{Network, Result},
};

use std::fmt::Debug;

#[async_trait]
pub trait LedgerService<N: Network>: Debug + Send + Sync {
    /// Returns the latest block height in the canonical ledger.
    fn latest_canon_height(&self) -> u32;

    /// Returns `true` if the given block height exists in the canonical ledger.
    fn contains_canon_height(&self, height: u32) -> bool;

    /// Returns the canonical block height for the given block hash, if it exists.
    fn get_canon_height(&self, hash: &N::BlockHash) -> Option<u32>;

    /// Returns the canonical block hash for the given block height, if it exists.
    fn get_canon_hash(&self, height: u32) -> Option<N::BlockHash>;

    /// Checks the given block is valid next block.
    fn check_next_block(&self, block: &Block<N>) -> Result<()>;

    /// Adds the given block as the next block in the ledger.
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()>;
}
