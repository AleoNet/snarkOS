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

use crate::{
    helpers::{Pending, Ready},
    Gateway,
    Shared,
};
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{ProverSolution, PuzzleCommitment, Transaction},
};

use std::sync::Arc;

fn fmt_id(id: String) -> String {
    id.chars().take(12).collect::<String>()
}

#[derive(Clone)]
pub struct Worker<N: Network> {
    /// The worker ID.
    id: u8,
    /// The shared state.
    shared: Arc<Shared<N>>,
    /// The gateway.
    gateway: Gateway<N>,
    /// The ready queue.
    ready: Ready<N>,
    /// The pending queue.
    pending: Pending<N>,
}

impl<N: Network> Worker<N> {
    /// Initializes a new worker instance.
    pub fn new(id: u8, shared: Arc<Shared<N>>, gateway: Gateway<N>) -> Result<Self> {
        // Return the worker.
        Ok(Self { id, shared, gateway, ready: Default::default(), pending: Default::default() })
    }

    /// Run the worker instance.
    pub async fn run(&mut self) -> Result<(), Error> {
        info!("Starting worker instance {} of the memory pool...", self.id);

        // // Create the validator instance.
        // let mut validator = Validator::new(self.shared.clone());
        //
        // // Run the validator instance.
        // validator.run().await?;

        Ok(())
    }

    /// Handles the incoming unconfirmed solution.
    /// Note: This method assumes the incoming solution is valid; it is the caller's responsibility.
    pub(crate) async fn process_unconfirmed_solution(
        &self,
        (puzzle_commitment, prover_solution): (PuzzleCommitment<N>, Data<ProverSolution<N>>),
    ) -> Result<()> {
        trace!("Worker {} - Unconfirmed solution '{}'", self.id, fmt_id(puzzle_commitment.to_string()));
        // Remove the puzzle commitment from the pending queue.
        self.pending.remove(puzzle_commitment);
        // Adds the prover solution to the ready queue.
        self.ready.insert(puzzle_commitment, prover_solution);
        Ok(())
    }

    /// Handles the incoming unconfirmed transaction.
    /// Note: This method assumes the incoming transaction is valid; it is the caller's responsibility.
    pub(crate) async fn process_unconfirmed_transaction(
        &self,
        (transaction_id, transaction): (N::TransactionID, Data<Transaction<N>>),
    ) -> Result<()> {
        trace!("Worker {} - Unconfirmed transaction '{}'", self.id, fmt_id(transaction_id.to_string()));
        // Remove the transaction from the pending queue.
        self.pending.remove(&transaction_id);
        // Adds the transaction to the ready queue.
        self.ready.insert(&transaction_id, transaction);

        Ok(())
    }
}
