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
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        committee::Committee,
        narwhal::{Data, TransmissionID},
    },
    prelude::{Field, Network, Result},
};

use tracing::*;

/// A mock ledger service that always returns `false`.
#[derive(Debug)]
pub struct MockLedgerService<N: Network> {
    committee: Committee<N>,
}

impl<N: Network> MockLedgerService<N> {
    /// Initializes a new mock ledger service.
    pub fn new(committee: Committee<N>) -> Self {
        Self { committee }
    }
}

#[async_trait]
impl<N: Network> LedgerService<N> for MockLedgerService<N> {
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
