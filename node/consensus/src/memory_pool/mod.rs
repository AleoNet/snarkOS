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

mod solutions;
mod transactions;

use crate::Consensus;
use snarkvm::prelude::{
    anchor_block_height,
    block::Transaction,
    coinbase::{ProverSolution, PuzzleCommitment},
    store::ConsensusStorage,
    Itertools,
    Network,
};

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct MemoryPool<N: Network> {
    /// The pool of unconfirmed transactions.
    unconfirmed_transactions: Arc<RwLock<HashMap<N::TransactionID, Transaction<N>>>>,
    /// The pool of unconfirmed solutions and their proof targets.
    unconfirmed_solutions: Arc<RwLock<HashMap<PuzzleCommitment<N>, (ProverSolution<N>, u64)>>>,
}

impl<N: Network> MemoryPool<N> {
    /// Initializes a new instance of a memory pool.
    pub fn new() -> Self {
        Self { unconfirmed_transactions: Default::default(), unconfirmed_solutions: Default::default() }
    }
}

impl<N: Network> Default for MemoryPool<N> {
    fn default() -> Self {
        Self::new()
    }
}
