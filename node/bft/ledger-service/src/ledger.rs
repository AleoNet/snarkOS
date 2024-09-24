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

use crate::{fmt_id, spawn_blocking, LedgerService};
use snarkvm::{
    ledger::{
        block::{Block, Transaction},
        committee::Committee,
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
        puzzle::{Solution, SolutionID},
        store::ConsensusStorage,
        Ledger,
    },
    prelude::{bail, Address, Field, FromBytes, Network, Result},
};

use indexmap::IndexMap;
use lru::LruCache;
use parking_lot::{Mutex, RwLock};
use std::{
    fmt,
    io::Read,
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// The capacity of the LRU holding the recently queried committees.
const COMMITTEE_CACHE_SIZE: usize = 16;

/// A core ledger service.
#[allow(clippy::type_complexity)]
pub struct CoreLedgerService<N: Network, C: ConsensusStorage<N>> {
    ledger: Ledger<N, C>,
    committee_cache: Arc<Mutex<LruCache<u64, Committee<N>>>>,
    latest_leader: Arc<RwLock<Option<(u64, Address<N>)>>>,
    shutdown: Arc<AtomicBool>,
}

impl<N: Network, C: ConsensusStorage<N>> CoreLedgerService<N, C> {
    /// Initializes a new core ledger service.
    pub fn new(ledger: Ledger<N, C>, shutdown: Arc<AtomicBool>) -> Self {
        let committee_cache = Arc::new(Mutex::new(LruCache::new(COMMITTEE_CACHE_SIZE.try_into().unwrap())));
        Self { ledger, committee_cache, latest_leader: Default::default(), shutdown }
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
    /// Returns the latest round in the ledger.
    fn latest_round(&self) -> u64 {
        self.ledger.latest_round()
    }

    /// Returns the latest block height in the ledger.
    fn latest_block_height(&self) -> u32 {
        self.ledger.latest_height()
    }

    /// Returns the latest block in the ledger.
    fn latest_block(&self) -> Block<N> {
        self.ledger.latest_block()
    }

    /// Returns the latest restrictions ID in the ledger.
    fn latest_restrictions_id(&self) -> Field<N> {
        self.ledger.vm().restrictions().restrictions_id()
    }

    /// Returns the latest cached leader and its associated round.
    fn latest_leader(&self) -> Option<(u64, Address<N>)> {
        *self.latest_leader.read()
    }

    /// Updates the latest cached leader and its associated round.
    fn update_latest_leader(&self, round: u64, leader: Address<N>) {
        *self.latest_leader.write() = Some((round, leader));
    }

    /// Returns `true` if the given block height exists in the ledger.
    fn contains_block_height(&self, height: u32) -> bool {
        self.ledger.contains_block_height(height).unwrap_or(false)
    }

    /// Returns the block height for the given block hash, if it exists.
    fn get_block_height(&self, hash: &N::BlockHash) -> Result<u32> {
        self.ledger.get_height(hash)
    }

    /// Returns the block hash for the given block height, if it exists.
    fn get_block_hash(&self, height: u32) -> Result<N::BlockHash> {
        self.ledger.get_hash(height)
    }

    /// Returns the block round for the given block height, if it exists.
    fn get_block_round(&self, height: u32) -> Result<u64> {
        self.ledger.get_block(height).map(|block| block.round())
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

    /// Returns the solution for the given solution ID.
    fn get_solution(&self, solution_id: &SolutionID<N>) -> Result<Solution<N>> {
        self.ledger.get_solution(solution_id)
    }

    /// Returns the unconfirmed transaction for the given transaction ID.
    fn get_unconfirmed_transaction(&self, transaction_id: N::TransactionID) -> Result<Transaction<N>> {
        self.ledger.get_unconfirmed_transaction(&transaction_id)
    }

    /// Returns the batch certificate for the given batch certificate ID.
    fn get_batch_certificate(&self, certificate_id: &Field<N>) -> Result<BatchCertificate<N>> {
        match self.ledger.get_batch_certificate(certificate_id) {
            Ok(Some(certificate)) => Ok(certificate),
            Ok(None) => bail!("No batch certificate found for certificate ID {certificate_id} in the ledger"),
            Err(error) => Err(error),
        }
    }

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>> {
        self.ledger.latest_committee()
    }

    /// Returns the committee for the given round.
    fn get_committee_for_round(&self, round: u64) -> Result<Committee<N>> {
        // Check if the committee is already in the cache.
        if let Some(committee) = self.committee_cache.lock().get(&round) {
            return Ok(committee.clone());
        }

        match self.ledger.get_committee_for_round(round)? {
            // Return the committee if it exists.
            Some(committee) => {
                // Insert the committee into the cache.
                self.committee_cache.lock().push(round, committee.clone());
                // Return the committee.
                Ok(committee)
            }
            // Return the current committee if the round is equivalent.
            None => {
                // Retrieve the current committee.
                let current_committee = self.current_committee()?;
                // Return the current committee if the round is equivalent.
                match current_committee.starting_round() == round {
                    true => Ok(current_committee),
                    false => bail!("No committee found for round {round} in the ledger"),
                }
            }
        }
    }

    /// Returns the committee lookback for the given round.
    fn get_committee_lookback_for_round(&self, round: u64) -> Result<Committee<N>> {
        // Get the round number for the previous committee. Note, we subtract 2 from odd rounds,
        // because committees are updated in even rounds.
        let previous_round = match round % 2 == 0 {
            true => round.saturating_sub(1),
            false => round.saturating_sub(2),
        };

        // Get the committee lookback round.
        let committee_lookback_round = previous_round.saturating_sub(Committee::<N>::COMMITTEE_LOOKBACK_RANGE);

        // Retrieve the committee for the committee lookback round.
        self.get_committee_for_round(committee_lookback_round)
    }

    /// Returns `true` if the ledger contains the given certificate ID in block history.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool> {
        self.ledger.contains_certificate(certificate_id)
    }

    /// Returns `true` if the transmission exists in the ledger.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool> {
        match transmission_id {
            TransmissionID::Ratification => Ok(false),
            TransmissionID::Solution(solution_id, _) => self.ledger.contains_solution_id(solution_id),
            TransmissionID::Transaction(transaction_id, _) => self.ledger.contains_transaction_id(transaction_id),
        }
    }

    /// Ensures that the given transmission is not a fee and matches the given transmission ID.
    fn ensure_transmission_is_well_formed(
        &self,
        transmission_id: TransmissionID<N>,
        transmission: &mut Transmission<N>,
    ) -> Result<()> {
        match (transmission_id, transmission) {
            (TransmissionID::Ratification, Transmission::Ratification) => {}
            (
                TransmissionID::Transaction(expected_transaction_id, expected_checksum),
                Transmission::Transaction(transaction_data),
            ) => {
                // Deserialize the transaction. If the transaction exceeds the maximum size, then return an error.
                let transaction = match transaction_data.clone() {
                    Data::Object(transaction) => transaction,
                    Data::Buffer(bytes) => Transaction::<N>::read_le(&mut bytes.take(N::MAX_TRANSACTION_SIZE as u64))?,
                };
                // Ensure the transaction ID matches the expected transaction ID.
                if transaction.id() != expected_transaction_id {
                    bail!(
                        "Received mismatching transaction ID - expected {}, found {}",
                        fmt_id(expected_transaction_id),
                        fmt_id(transaction.id()),
                    );
                }

                // Ensure the transmission checksum matches the expected checksum.
                let checksum = transaction_data.to_checksum::<N>()?;
                if checksum != expected_checksum {
                    bail!(
                        "Received mismatching checksum for transaction {} - expected {expected_checksum} but found {checksum}",
                        fmt_id(expected_transaction_id)
                    );
                }

                // Ensure the transaction is not a fee transaction.
                if transaction.is_fee() {
                    bail!("Received a fee transaction in a transmission");
                }

                // Update the transmission with the deserialized transaction.
                *transaction_data = Data::Object(transaction);
            }
            (
                TransmissionID::Solution(expected_solution_id, expected_checksum),
                Transmission::Solution(solution_data),
            ) => {
                match solution_data.clone().deserialize_blocking() {
                    Ok(solution) => {
                        if solution.id() != expected_solution_id {
                            bail!(
                                "Received mismatching solution ID - expected {}, found {}",
                                fmt_id(expected_solution_id),
                                fmt_id(solution.id()),
                            );
                        }

                        // Ensure the transmission checksum matches the expected checksum.
                        let checksum = solution_data.to_checksum::<N>()?;
                        if checksum != expected_checksum {
                            bail!(
                                "Received mismatching checksum for solution {} - expected {expected_checksum} but found {checksum}",
                                fmt_id(expected_solution_id)
                            );
                        }

                        // Update the transmission with the deserialized solution.
                        *solution_data = Data::Object(solution);
                    }
                    Err(err) => {
                        bail!("Failed to deserialize solution: {err}");
                    }
                }
            }
            _ => {
                bail!("Mismatching `(transmission_id, transmission)` pair");
            }
        }

        Ok(())
    }

    /// Checks the given solution is well-formed.
    async fn check_solution_basic(&self, solution_id: SolutionID<N>, solution: Data<Solution<N>>) -> Result<()> {
        // Deserialize the solution.
        let solution = spawn_blocking!(solution.deserialize_blocking())?;
        // Ensure the solution ID matches in the solution.
        if solution_id != solution.id() {
            bail!("Invalid solution - expected {solution_id}, found {}", solution.id());
        }

        // Compute the current epoch hash.
        let epoch_hash = self.ledger.latest_epoch_hash()?;
        // Retrieve the current proof target.
        let proof_target = self.ledger.latest_proof_target();

        // Ensure that the solution is valid for the given epoch.
        let puzzle = self.ledger.puzzle().clone();
        match spawn_blocking!(puzzle.check_solution(&solution, epoch_hash, proof_target)) {
            Ok(()) => Ok(()),
            Err(e) => bail!("Invalid solution '{}' for the current epoch - {e}", fmt_id(solution_id)),
        }
    }

    /// Checks the given transaction is well-formed and unique.
    async fn check_transaction_basic(
        &self,
        transaction_id: N::TransactionID,
        transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        // Deserialize the transaction. If the transaction exceeds the maximum size, then return an error.
        let transaction = spawn_blocking!({
            match transaction {
                Data::Object(transaction) => Ok(transaction),
                Data::Buffer(bytes) => Ok(Transaction::<N>::read_le(&mut bytes.take(N::MAX_TRANSACTION_SIZE as u64))?),
            }
        })?;
        // Ensure the transaction ID matches in the transaction.
        if transaction_id != transaction.id() {
            bail!("Invalid transaction - expected {transaction_id}, found {}", transaction.id());
        }
        // Check if the transmission is a fee transaction.
        if transaction.is_fee() {
            bail!("Invalid transaction - 'Transaction::fee' type is not valid at this stage ({})", transaction.id());
        }
        // Check the transaction is well-formed.
        let ledger = self.ledger.clone();
        spawn_blocking!(ledger.check_transaction_basic(&transaction, None, &mut rand::thread_rng()))
    }

    /// Checks the given block is valid next block.
    fn check_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.check_next_block(block, &mut rand::thread_rng())
    }

    /// Returns a candidate for the next block in the ledger, using a committed subdag and its transmissions.
    #[cfg(feature = "ledger-write")]
    fn prepare_advance_to_next_quorum_block(
        &self,
        subdag: Subdag<N>,
        transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<Block<N>> {
        self.ledger.prepare_advance_to_next_quorum_block(subdag, transmissions, &mut rand::thread_rng())
    }

    /// Adds the given block as the next block in the ledger.
    #[cfg(feature = "ledger-write")]
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        // If the Ctrl-C handler registered the signal, then skip advancing to the next block.
        if self.shutdown.load(Ordering::Relaxed) {
            bail!("Skipping advancing to block {} - The node is shutting down", block.height());
        }
        // Advance to the next block.
        self.ledger.advance_to_next_block(block)?;
        // Update BFT metrics.
        #[cfg(feature = "metrics")]
        {
            let num_sol = block.solutions().len();
            let num_tx = block.transactions().len();

            metrics::gauge(metrics::bft::HEIGHT, block.height() as f64);
            metrics::gauge(metrics::bft::LAST_COMMITTED_ROUND, block.round() as f64);
            metrics::increment_gauge(metrics::blocks::SOLUTIONS, num_sol as f64);
            metrics::increment_gauge(metrics::blocks::TRANSACTIONS, num_tx as f64);
            metrics::update_block_metrics(block);
        }

        tracing::info!("\n\nAdvanced to block {} at round {} - {}\n", block.height(), block.round(), block.hash());
        Ok(())
    }
}
