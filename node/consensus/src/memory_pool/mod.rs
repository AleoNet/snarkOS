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

mod solutions;
mod transactions;

use crate::{anchor_block_height, Consensus};
use snarkvm::prelude::{ConsensusStorage, Itertools, Network, ProverSolution, PuzzleCommitment, Transaction};

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

impl<N: Network> Default for MemoryPool<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> MemoryPool<N> {
    /// Initializes a new instance of a memory pool.
    pub fn new() -> Self {
        Self { unconfirmed_transactions: Default::default(), unconfirmed_solutions: Default::default() }
    }
}
