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
    ledger::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        narwhal::Data,
    },
    prelude::{narwhal::TransmissionID, Field, Network, Result},
};

#[async_trait]
pub trait LedgerService<N: Network>: Send + Sync {
    /// Returns `true` if the ledger contains the given certificate ID.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool>;

    /// Returns `true` if the ledger contains the given transmission ID.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool>;

    /// Checks the given solution is well-formed.
    async fn check_solution_basic(
        &self,
        puzzle_commitment: PuzzleCommitment<N>,
        solution: Data<ProverSolution<N>>,
    ) -> Result<()>;

    /// Checks the given transaction is well-formed and unique.
    async fn check_transaction_basic(
        &self,
        transaction_id: N::TransactionID,
        transaction: Data<Transaction<N>>,
    ) -> Result<()>;
}
