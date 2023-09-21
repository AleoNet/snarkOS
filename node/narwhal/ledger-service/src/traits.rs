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
        block::{Block, Transaction},
        coinbase::{ProverSolution, PuzzleCommitment},
        committee::Committee,
        narwhal::{Data, Subdag, Transmission, TransmissionID},
    },
    prelude::{Field, Network, Result},
};

use indexmap::IndexMap;
use std::fmt::Debug;

#[async_trait]
pub trait LedgerService<N: Network>: Debug + Send + Sync {
    /// Returns the latest block height in the canonical ledger.
    fn latest_block_height(&self) -> u32;

    /// Returns `true` if the given block height exists in the canonical ledger.
    fn contains_block_height(&self, height: u32) -> bool;

    /// Returns the canonical block height for the given block hash, if it exists.
    fn get_block_height(&self, hash: &N::BlockHash) -> Option<u32>;

    /// Returns the canonical block hash for the given block height, if it exists.
    fn get_block_hash(&self, height: u32) -> Option<N::BlockHash>;

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>>;

    /// Returns the committee for the given round.
    /// If the given round is in the future, then the current committee is returned.
    fn get_committee_for_round(&self, round: u64) -> Result<Committee<N>>;

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

    /// Checks the given block is valid next block.
    fn check_next_block(&self, block: &Block<N>) -> Result<()>;

    /// Returns a candidate for the next block in the ledger, using a committed subdag and its transmissions.
    #[cfg(feature = "ledger-write")]
    fn prepare_advance_to_next_quorum_block(
        &self,
        subdag: Subdag<N>,
        transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<Block<N>>;

    /// Adds the given block as the next block in the ledger.
    #[cfg(feature = "ledger-write")]
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()>;
}
