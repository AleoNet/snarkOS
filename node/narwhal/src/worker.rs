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
    helpers::{Pending, Ready, WorkerReceiver},
    Event,
    Gateway,
    Ping,
    Shared,
    MAX_WORKERS,
};
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{ProverSolution, PuzzleCommitment, Transaction},
};

use parking_lot::Mutex;
use std::{future::Future, net::SocketAddr, sync::Arc};
use tokio::task::JoinHandle;

fn fmt_id(id: String) -> String {
    id.chars().take(16).collect::<String>()
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
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Worker<N> {
    /// Initializes a new worker instance.
    pub fn new(id: u8, shared: Arc<Shared<N>>, gateway: Gateway<N>) -> Result<Self> {
        // Ensure the worker ID is valid.
        ensure!(id < MAX_WORKERS, "Invalid worker ID '{id}'");
        // Return the worker.
        Ok(Self {
            id,
            shared,
            gateway,
            ready: Default::default(),
            pending: Default::default(),
            handles: Default::default(),
        })
    }

    /// Returns the worker ID.
    pub const fn id(&self) -> u8 {
        self.id
    }

    /// Run the worker instance.
    pub async fn run(&mut self, receiver: WorkerReceiver<N>) -> Result<(), Error> {
        info!("Starting worker instance {} of the memory pool...", self.id);

        // Start the worker handlers.
        self.start_handlers(receiver);

        Ok(())
    }

    /// Starts the worker handlers.
    pub fn start_handlers(&self, receiver: WorkerReceiver<N>) {
        let WorkerReceiver { mut rx_ping } = receiver;

        // Broadcast a ping event periodically.
        let self_clone = self.clone();
        self.spawn(async move {
            loop {
                // Broadcast the ping event.
                self_clone.broadcast_ping().await;
                // Wait for the next interval.
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        });

        // Process the ping events.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, ping)) = rx_ping.recv().await {
                // Process the ping event.
                self_clone.process_ping(peer_ip, ping).await;
            }
        });
    }

    /// Broadcasts a ping event.
    pub(crate) async fn broadcast_ping(&self) {
        // Construct the ping event.
        let ping = Ping::new(self.id, self.ready.entry_ids());
        // Broadcast the ping event.
        self.gateway.broadcast(Event::Ping(ping));
    }

    /// Handles the incoming ping event.
    pub(crate) async fn process_ping(&self, peer_ip: SocketAddr, ping: Ping<N>) {
        // Ensure the ping is for this worker.
        if ping.worker != self.id {
            return;
        }

        // Iterate through the batch.
        for entry_id in &ping.batch {
            // Check if the entry ID exists in the ready queue.
            if self.ready.contains(*entry_id) {
                continue;
            }
            // Check if the entry ID exists in the pending queue.
            if !self.pending.contains(*entry_id) {
                // TODO (howardwu): Send a request to the peer to fetch the entry.
            }
            // Check if the entry ID exists in the pending queue for the specified peer IP.
            if !self.pending.contains_peer(*entry_id, peer_ip) {
                // Insert the entry ID into the pending queue.
                self.pending.insert(*entry_id, peer_ip);
            }
        }
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

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    pub fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the worker.
    pub fn shut_down(&self) {
        trace!("Shutting down worker {}...", self.id);
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}
