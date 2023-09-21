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
    ledger::{
        block::{Block, Transaction},
        coinbase::{ProverSolution, PuzzleCommitment},
        committee::Committee,
        narwhal::{Data, Subdag, Transmission, TransmissionID},
        store::ConsensusStorage,
        Ledger,
    },
    prelude::{bail, Field, Network, Result},
};

use indexmap::IndexMap;
use std::{fmt, ops::Range};

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
        f.debug_struct("CoreLedgerService").field("current_committee", &self.current_committee()).finish()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> LedgerService<N> for CoreLedgerService<N, C> {
    /// Returns the latest block height in the ledger.
    fn latest_block_height(&self) -> u32 {
        self.ledger.latest_height()
    }

    /// Returns `true` if the given block height exists in the ledger.
    fn contains_block_height(&self, height: u32) -> bool {
        self.ledger.contains_block_height(height).unwrap_or(false)
    }

    /// Returns the block height for the given block hash, if it exists.
    fn get_block_height(&self, hash: &N::BlockHash) -> Option<u32> {
        self.ledger.get_height(hash).ok()
    }

    /// Returns the block hash for the given block height, if it exists.
    fn get_block_hash(&self, height: u32) -> Option<N::BlockHash> {
        self.ledger.get_hash(height).ok()
    }

    /// Returns the block for the given block height.
    fn get_block(&self, height: u32) -> Result<Block<N>> {
        self.ledger.get_block(height)
    }

    /// Returns the blocks in the given block range.
    /// The range is inclusive of the start and exclusive of the end.
    fn get_blocks(&self, heights: Range<u32>) -> Result<Vec<Block<N>>> {
        self.ledger.get_blocks(heights)
    }

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>> {
        self.ledger.latest_committee()
    }

    /// Returns the committee for the given round.
    /// If the given round is in the future, then the current committee is returned.
    fn get_committee_for_round(&self, round: u64) -> Result<Committee<N>> {
        match self.ledger.get_committee_for_round(round)? {
            // Return the committee if it exists.
            Some(committee) => Ok(committee),
            // Return the current committee if the round is in the future.
            None => {
                // Retrieve the current committee.
                let current_committee = self.current_committee()?;
                // Return the current committee if the round is in the future.
                match current_committee.starting_round() <= round {
                    true => Ok(current_committee),
                    false => bail!("No committee found for round {round} in the ledger"),
                }
            }
        }
    }

    /// Returns `true` if the ledger contains the given certificate ID in block history.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool> {
        self.ledger.contains_certificate(certificate_id)
    }

    /// Returns `true` if the transmission exists in the ledger.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool> {
        match transmission_id {
            TransmissionID::Ratification => Ok(false),
            TransmissionID::Solution(puzzle_commitment) => self.ledger.contains_puzzle_commitment(puzzle_commitment),
            TransmissionID::Transaction(transaction_id) => self.ledger.contains_transaction_id(transaction_id),
        }
    }

    /// Checks the given solution is well-formed.
    async fn check_solution_basic(
        &self,
        puzzle_commitment: PuzzleCommitment<N>,
        solution: Data<ProverSolution<N>>,
    ) -> Result<()> {
        // Deserialize the solution.
        let solution = tokio::task::spawn_blocking(move || solution.deserialize_blocking()).await??;
        // Ensure the puzzle commitment matches in the solution.
        if puzzle_commitment != solution.commitment() {
            bail!("Invalid solution - expected {puzzle_commitment}, found {}", solution.commitment());
        }

        // Retrieve the coinbase verifying key.
        let coinbase_verifying_key = self.ledger.coinbase_puzzle().coinbase_verifying_key();
        // Compute the current epoch challenge.
        let epoch_challenge = self.ledger.latest_epoch_challenge()?;
        // Retrieve the current proof target.
        let proof_target = self.ledger.latest_proof_target();

        // Ensure that the prover solution is valid for the given epoch.
        // TODO(ljedrz): check if this operation requires a blocking task.
        if !solution.verify(coinbase_verifying_key, &epoch_challenge, proof_target)? {
            bail!("Invalid prover solution '{puzzle_commitment}' for the current epoch.");
        }
        Ok(())
    }

    /// Checks the given transaction is well-formed and unique.
    async fn check_transaction_basic(
        &self,
        transaction_id: N::TransactionID,
        transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        // Deserialize the transaction.
        let transaction = tokio::task::spawn_blocking(move || transaction.deserialize_blocking()).await??;
        // Ensure the transaction ID matches in the transaction.
        if transaction_id != transaction.id() {
            bail!("Invalid transaction - expected {transaction_id}, found {}", transaction.id());
        }
        // Check the transaction is well-formed.
        // TODO(ljedrz): check if this operation requires a blocking task.
        self.ledger.check_transaction_basic(&transaction, None)
    }

    /// Checks the given block is valid next block.
    fn check_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.check_next_block(block)
    }

    /// Returns a candidate for the next block in the ledger, using a committed subdag and its transmissions.
    #[cfg(feature = "ledger-write")]
    fn prepare_advance_to_next_quorum_block(
        &self,
        subdag: Subdag<N>,
        transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<Block<N>> {
        self.ledger.prepare_advance_to_next_quorum_block(subdag, transmissions)
    }

    /// Adds the given block as the next block in the ledger.
    #[cfg(feature = "ledger-write")]
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.advance_to_next_block(block)
    }
}
