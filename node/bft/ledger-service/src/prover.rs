// Copyright 2024 Aleo Network Foundation
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
    ledger::{
        block::{Block, Transaction},
        committee::Committee,
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
        puzzle::{Solution, SolutionID},
    },
    prelude::{bail, Address, Field, Network, Result, Zero},
};

use indexmap::IndexMap;
use std::ops::Range;

/// A ledger service for a prover.
#[derive(Clone, Debug, Default)]
pub struct ProverLedgerService<N: Network> {
    _network: std::marker::PhantomData<N>,
}

impl<N: Network> ProverLedgerService<N> {
    /// Initializes a new prover ledger service.
    pub fn new() -> Self {
        Self { _network: Default::default() }
    }
}

#[async_trait]
impl<N: Network> LedgerService<N> for ProverLedgerService<N> {
    /// Returns the latest round in the ledger.
    fn latest_round(&self) -> u64 {
        0u64
    }

    /// Returns the latest block height in the ledger.
    fn latest_block_height(&self) -> u32 {
        0u32
    }

    /// Returns the latest block in the ledger.
    fn latest_block(&self) -> Block<N> {
        unreachable!("Latest block does not exist in prover")
    }

    /// Returns the latest restrictions ID in the ledger.
    fn latest_restrictions_id(&self) -> Field<N> {
        Field::zero()
    }

    /// Returns the latest cached leader and its associated round.
    fn latest_leader(&self) -> Option<(u64, Address<N>)> {
        unreachable!("Latest leader does not exist in prover");
    }

    /// Updates the latest cached leader and its associated round.
    fn update_latest_leader(&self, _round: u64, _leader: Address<N>) {
        unreachable!("Latest leader does not exist in prover");
    }

    /// Returns `true` if the given block height exists in the ledger.
    fn contains_block_height(&self, _height: u32) -> bool {
        false
    }

    /// Returns the block height for the given block hash, if it exists.
    fn get_block_height(&self, hash: &N::BlockHash) -> Result<u32> {
        bail!("Block hash '{hash}' does not exist in prover")
    }

    /// Returns the block hash for the given block height, if it exists.
    fn get_block_hash(&self, height: u32) -> Result<N::BlockHash> {
        bail!("Block {height} does not exist in prover")
    }

    /// Returns the block round for the given block height, if it exists.
    fn get_block_round(&self, height: u32) -> Result<u64> {
        bail!("Block {height} does not exist in prover")
    }

    /// Returns the block for the given block height.
    fn get_block(&self, height: u32) -> Result<Block<N>> {
        bail!("Block {height} does not exist in prover")
    }

    /// Returns the blocks in the given block range.
    /// The range is inclusive of the start and exclusive of the end.
    fn get_blocks(&self, heights: Range<u32>) -> Result<Vec<Block<N>>> {
        bail!("Blocks {heights:?} do not exist in prover")
    }

    /// Returns the solution for the given solution ID.
    fn get_solution(&self, solution_id: &SolutionID<N>) -> Result<Solution<N>> {
        bail!("Solution '{solution_id}' does not exist in prover")
    }

    /// Returns the unconfirmed transaction for the given transaction ID.
    fn get_unconfirmed_transaction(&self, transaction_id: N::TransactionID) -> Result<Transaction<N>> {
        bail!("Transaction '{transaction_id}' does not exist in prover")
    }

    /// Returns the batch certificate for the given batch certificate ID.
    fn get_batch_certificate(&self, certificate_id: &Field<N>) -> Result<BatchCertificate<N>> {
        bail!("Batch certificate '{certificate_id}' does not exist in prover")
    }

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>> {
        bail!("Committee does not exist in prover")
    }

    /// Returns the committee for the given round.
    fn get_committee_for_round(&self, round: u64) -> Result<Committee<N>> {
        bail!("Committee for round {round} does not exist in prover")
    }

    /// Returns the committee lookback for the given round.
    fn get_committee_lookback_for_round(&self, round: u64) -> Result<Committee<N>> {
        bail!("Previous committee for round {round} does not exist in prover")
    }

    /// Returns `true` if the ledger contains the given certificate ID in block history.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool> {
        bail!("Certificate '{certificate_id}' does not exist in prover")
    }

    /// Returns `true` if the transmission exists in the ledger.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool> {
        bail!("Transmission '{transmission_id}' does not exist in prover")
    }

    /// Ensures that the given transmission is not a fee and matches the given transmission ID.
    fn ensure_transmission_is_well_formed(
        &self,
        _transmission_id: TransmissionID<N>,
        _transmission: &mut Transmission<N>,
    ) -> Result<()> {
        Ok(())
    }

    /// Checks the given solution is well-formed.
    async fn check_solution_basic(&self, _solution_id: SolutionID<N>, _solution: Data<Solution<N>>) -> Result<()> {
        Ok(())
    }

    /// Checks the given transaction is well-formed and unique.
    async fn check_transaction_basic(
        &self,
        _transaction_id: N::TransactionID,
        _transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        Ok(())
    }

    /// Checks the given block is valid next block.
    fn check_next_block(&self, _block: &Block<N>) -> Result<()> {
        Ok(())
    }

    /// Returns a candidate for the next block in the ledger, using a committed subdag and its transmissions.
    #[cfg(feature = "ledger-write")]
    fn prepare_advance_to_next_quorum_block(
        &self,
        _subdag: Subdag<N>,
        _transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<Block<N>> {
        bail!("Cannot prepare advance to next quorum block in prover")
    }

    /// Adds the given block as the next block in the ledger.
    #[cfg(feature = "ledger-write")]
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        bail!("Cannot advance to next block in prover - {block}")
    }
}
