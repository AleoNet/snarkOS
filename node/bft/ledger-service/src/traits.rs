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
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
    },
    prelude::{Address, Field, Network, Result},
};

use indexmap::IndexMap;
use std::{fmt::Debug, ops::Range};

#[async_trait]
pub trait LedgerService<N: Network>: Debug + Send + Sync {
    /// Returns the latest round in the ledger.
    fn latest_round(&self) -> u64;

    /// Returns the latest block height in the ledger.
    fn latest_block_height(&self) -> u32;

    /// Returns the latest block in the ledger.
    fn latest_block(&self) -> Block<N>;

    /// Returns the latest cached leader and its associated round.
    fn latest_leader(&self) -> Option<(u64, Address<N>)>;

    /// Updates the latest cached leader and its associated round.
    fn update_latest_leader(&self, round: u64, leader: Address<N>);

    /// Returns `true` if the given block height exists in the ledger.
    fn contains_block_height(&self, height: u32) -> bool;

    /// Returns the block height for the given block hash, if it exists.
    fn get_block_height(&self, hash: &N::BlockHash) -> Result<u32>;

    /// Returns the block hash for the given block height, if it exists.
    fn get_block_hash(&self, height: u32) -> Result<N::BlockHash>;

    /// Returns the block round for the given block height, if it exists.
    fn get_block_round(&self, height: u32) -> Result<u64>;

    /// Returns the block for the given block height.
    fn get_block(&self, height: u32) -> Result<Block<N>>;

    /// Returns the blocks in the given block range.
    /// The range is inclusive of the start and exclusive of the end.
    fn get_blocks(&self, heights: Range<u32>) -> Result<Vec<Block<N>>>;

    /// Returns the solution for the given solution ID.
    fn get_solution(&self, solution_id: &PuzzleCommitment<N>) -> Result<ProverSolution<N>>;

    /// Returns the unconfirmed transaction for the given transaction ID.
    fn get_unconfirmed_transaction(&self, transaction_id: N::TransactionID) -> Result<Transaction<N>>;

    /// Returns the batch certificate for the given batch certificate ID.
    fn get_batch_certificate(&self, certificate_id: &Field<N>) -> Result<BatchCertificate<N>>;

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>>;

    /// Returns the committee for the given round.
    /// If the given round is in the future, then the current committee is returned.
    fn get_committee_for_round(&self, round: u64) -> Result<Committee<N>>;

    /// Returns the committee lookback for the given round.
    /// If the committee lookback round is in the future, then the current committee is returned.
    fn get_committee_lookback_for_round(&self, round: u64) -> Result<Committee<N>>;

    /// Returns `true` if the ledger contains the given certificate ID.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool>;

    /// Returns `true` if the ledger contains the given transmission ID.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool>;

    /// Ensures that the given transmission is not a fee and matches the given transmission ID.
    fn ensure_transmission_is_well_formed(
        &self,
        transmission_id: TransmissionID<N>,
        transmission: &mut Transmission<N>,
    ) -> Result<()>;

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
