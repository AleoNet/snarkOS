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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

mod memory_pool;
pub use memory_pool::*;

#[cfg(test)]
mod tests;

use snarkvm::prelude::*;

use anyhow::Result;

#[derive(Clone)]
pub struct Consensus<N: Network, C: ConsensusStorage<N>> {
    /// The ledger.
    ledger: Ledger<N, C>,
    /// The memory pool.
    memory_pool: MemoryPool<N>,
    /// The boolean flag for the development mode.
    #[allow(dead_code)]
    is_dev: bool,
}

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Initializes a new instance of consensus.
    pub fn new(ledger: Ledger<N, C>, is_dev: bool) -> Result<Self> {
        Ok(Self { ledger, memory_pool: Default::default(), is_dev })
    }

    /// Returns the ledger.
    pub const fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
    }

    /// Returns the coinbase puzzle.
    pub const fn coinbase_puzzle(&self) -> &CoinbasePuzzle<N> {
        self.ledger.coinbase_puzzle()
    }

    /// Returns the memory pool.
    pub const fn memory_pool(&self) -> &MemoryPool<N> {
        &self.memory_pool
    }

    /// Checks the given transaction is well-formed and unique.
    pub fn check_transaction_basic(&self, transaction: &Transaction<N>, rejected_id: Option<Field<N>>) -> Result<()> {
        self.ledger.check_transaction_basic(transaction, rejected_id)
    }

    /// Checks the given block is valid next block.
    pub fn check_next_block(&self, block: &Block<N>) -> Result<()> {
        self.ledger.check_next_block(block)
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub fn add_unconfirmed_transaction(&self, transaction: Transaction<N>) -> Result<()> {
        // Ensure the transaction is not already in the memory pool.
        if self.memory_pool.contains_unconfirmed_transaction(transaction.id()) {
            bail!("Transaction is already in the memory pool.");
        }
        // Check that the transaction is well-formed and unique.
        self.check_transaction_basic(&transaction, None)?;
        // Insert the transaction to the memory pool.
        self.memory_pool.add_unconfirmed_transaction(&transaction);

        Ok(())
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&self, solution: &ProverSolution<N>) -> Result<()> {
        // Ensure the prover solution is not already in the memory pool.
        if self.memory_pool.contains_unconfirmed_solution(solution.commitment()) {
            bail!("Prover solution is already in the memory pool.");
        }
        // Ensure the prover solution is not already in the ledger.
        if self.ledger.contains_puzzle_commitment(&solution.commitment())? {
            bail!("Prover solution is already in the ledger.");
        }

        // Compute the current epoch challenge.
        let epoch_challenge = self.ledger.latest_epoch_challenge()?;
        // Retrieve the current proof target.
        let proof_target = self.ledger.latest_proof_target();

        // Ensure that the prover solution is valid for the given epoch.
        if !solution.verify(self.coinbase_puzzle().coinbase_verifying_key(), &epoch_challenge, proof_target)? {
            bail!("Invalid prover solution '{}' for the current epoch.", solution.commitment());
        }

        // Insert the solution to the memory pool.
        self.memory_pool.add_unconfirmed_solution(solution)?;

        Ok(())
    }

    /// Returns `true` if the coinbase target is met.
    pub fn is_coinbase_target_met(&self) -> Result<bool> {
        // Retrieve the latest proof target.
        let latest_proof_target = self.ledger.latest_proof_target();
        // Compute the candidate coinbase target.
        let cumulative_proof_target = self.memory_pool.candidate_coinbase_target(latest_proof_target)?;
        // Retrieve the latest coinbase target.
        let latest_coinbase_target = self.ledger.latest_coinbase_target();
        // Check if the coinbase target is met.
        Ok(cumulative_proof_target >= latest_coinbase_target as u128)
    }

    /// Returns a candidate for the next block in the ledger.
    pub fn propose_next_block<R: Rng + CryptoRng>(&self, private_key: &PrivateKey<N>, rng: &mut R) -> Result<Block<N>> {
        // Retrieve the latest block.
        let latest_block = self.ledger.latest_block();
        // Retrieve the latest height.
        let latest_height = latest_block.height();
        // Retrieve the latest proof target.
        let latest_proof_target = latest_block.proof_target();
        // Retrieve the latest coinbase target.
        let latest_coinbase_target = latest_block.coinbase_target();

        // Select the transactions from the memory pool.
        let transactions = self.memory_pool.candidate_transactions(self);
        // Select the prover solutions from the memory pool.
        let prover_solutions =
            self.memory_pool.candidate_solutions(self, latest_height, latest_proof_target, latest_coinbase_target)?;

        // Prepare the next block.
        self.ledger.prepare_advance_to_next_block(private_key, transactions, prover_solutions, rng)
    }

    /// Advances the ledger to the next block.
    pub fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        // Adds the next block to the ledger.
        let old_epoch = self.ledger.latest_epoch_number();
        self.ledger.advance_to_next_block(block)?;

        // Clear the memory pool of unconfirmed transactions that are now invalid.
        self.memory_pool.clear_invalid_transactions(self);

        // If this starts a new epoch, clear all unconfirmed solutions from the memory pool.
        if block.epoch_number() > old_epoch {
            self.memory_pool.clear_all_unconfirmed_solutions();
        }
        // Otherwise, if a new coinbase was produced, clear the memory pool of unconfirmed solutions that are now invalid.
        else if block.coinbase().is_some() {
            self.memory_pool.clear_invalid_solutions(self);
        }

        info!("Advanced to block {}", block.height());
        Ok(())
    }

    /// Clears the memory pool of invalid solutions and transactions.
    pub fn refresh_memory_pool(&self) -> Result<()> {
        // Clear the memory pool of unconfirmed solutions that are now invalid.
        self.memory_pool.clear_invalid_solutions(self);
        // Clear the memory pool of unconfirmed transactions that are now invalid.
        self.memory_pool.clear_invalid_transactions(self);
        Ok(())
    }

    /// Clears the memory pool of all solutions and transactions.
    pub fn clear_memory_pool(&self) -> Result<()> {
        // Clear the memory pool of unconfirmed solutions that are now invalid.
        self.memory_pool.clear_all_unconfirmed_solutions();
        // Clear the memory pool of unconfirmed transactions that are now invalid.
        self.memory_pool.clear_unconfirmed_transactions();
        Ok(())
    }
}
