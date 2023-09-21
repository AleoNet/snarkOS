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

use crate::{fmt_id, LedgerService};
use snarkvm::{
    ledger::{
        block::{Block, Transaction},
        coinbase::{ProverSolution, PuzzleCommitment},
        committee::Committee,
        narwhal::{Data, TransmissionID},
    },
    prelude::{ensure, Field, Network, Result},
};

use parking_lot::Mutex;
use std::collections::BTreeMap;
use tracing::*;

/// A mock ledger service that always returns `false`.
#[derive(Debug)]
pub struct MockLedgerService<N: Network> {
    committee: Committee<N>,
    height_to_hash: Mutex<BTreeMap<u32, N::BlockHash>>,
}

impl<N: Network> MockLedgerService<N> {
    /// Initializes a new mock ledger service.
    pub fn new(committee: Committee<N>) -> Self {
        Self { committee, height_to_hash: Default::default() }
    }

    /// Initializes a new mock ledger service at the specified height.
    pub fn new_at_height(committee: Committee<N>, height: u32) -> Self {
        let mut height_to_hash = BTreeMap::new();
        for i in 0..=height {
            height_to_hash.insert(i, (Field::<N>::from_u32(i)).into());
        }
        Self { committee, height_to_hash: Mutex::new(height_to_hash) }
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

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>> {
        Ok(self.committee.clone())
    }

    /// Returns the committee for the given round.
    /// If the given round is in the future, then the current committee is returned.
    fn get_committee_for_round(&self, _round: u64) -> Result<Committee<N>> {
        Ok(self.committee.clone())
    }

    /// Returns `false` for all queries.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool> {
        trace!("[MockLedgerService] Contains certificate ID {} - false", fmt_id(certificate_id));
        Ok(false)
    }

    /// Returns `false` for all queries.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool> {
        trace!("[MockLedgerService] Contains transmission ID {} - false", fmt_id(transmission_id));
        Ok(false)
    }

    /// Checks the given solution is well-formed.
    async fn check_solution_basic(
        &self,
        puzzle_commitment: PuzzleCommitment<N>,
        _solution: Data<ProverSolution<N>>,
    ) -> Result<()> {
        trace!("[MockLedgerService] Check solution basic {:?} - Ok", fmt_id(puzzle_commitment));
        Ok(())
    }

    /// Checks the given transaction is well-formed and unique.
    async fn check_transaction_basic(
        &self,
        transaction_id: N::TransactionID,
        _transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        trace!("[MockLedgerService] Check transaction basic {:?} - Ok", fmt_id(transaction_id));
        Ok(())
    }
}
