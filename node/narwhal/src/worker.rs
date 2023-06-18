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
    helpers::{Entry, EntryID, Pending, Ready, WorkerReceiver},
    EntryRequest,
    EntryResponse,
    Event,
    Gateway,
    Shared,
    WorkerPing,
    MAX_WORKERS,
};
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{ProverSolution, PuzzleCommitment, Transaction},
};

use parking_lot::Mutex;
use std::{collections::HashMap, future::Future, net::SocketAddr, sync::Arc};
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

    /// Transitions the worker to the next round.
    pub(crate) fn next_round(&self) -> HashMap<EntryID<N>, Data<Entry<N>>> {
        self.ready.drain()
    }
}

impl<N: Network> Worker<N> {
    /// Starts the worker handlers.
    pub fn start_handlers(&self, receiver: WorkerReceiver<N>) {
        let WorkerReceiver { rx_worker_ping: mut rx_ping, mut rx_entry_request, mut rx_entry_response } = receiver;

        // Broadcast a ping event periodically.
        let self_clone = self.clone();
        self.spawn(async move {
            loop {
                // Broadcast the ping event.
                self_clone.broadcast_ping().await;
                // Wait for the next interval.
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });

        // Process the ping events.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, ping)) = rx_ping.recv().await {
                // Process the ping event.
                self_clone.process_worker_ping(peer_ip, ping).await;
            }
        });

        // Process the entry requests.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, entry_request)) = rx_entry_request.recv().await {
                // Process the entry request.
                self_clone.process_entry_request(peer_ip, entry_request).await;
            }
        });

        // Process the entry responses.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, entry_response)) = rx_entry_response.recv().await {
                // Process the entry response.
                if let Err(e) = self_clone.process_entry_response(peer_ip, entry_response).await {
                    error!("Worker {} failed to process entry response from peer '{peer_ip}': {e}", self_clone.id);
                }
            }
        });
    }

    /// Broadcasts a ping event.
    pub(crate) async fn broadcast_ping(&self) {
        // Construct the ping event.
        let ping = WorkerPing::new(self.id, self.ready.entry_ids());
        // Broadcast the ping event.
        self.gateway.broadcast(Event::WorkerPing(ping));
    }

    /// Sends an entry request to the specified peer.
    pub(crate) async fn send_entry_request(&self, peer_ip: SocketAddr, entry_id: EntryID<N>) {
        // Construct the entry request.
        let entry_request = EntryRequest::new(self.id, entry_id);
        // Send the entry request to the peer.
        self.gateway.send(peer_ip, Event::EntryRequest(entry_request));
    }

    /// Sends an entry response to the specified peer.
    pub(crate) async fn send_entry_response(&self, peer_ip: SocketAddr, entry_id: EntryID<N>, entry: Data<Entry<N>>) {
        // Construct the entry response.
        let entry_response = EntryResponse::new(self.id, entry_id, entry);
        // Send the entry response to the peer.
        self.gateway.send(peer_ip, Event::EntryResponse(entry_response));
    }

    /// Handles the incoming ping event.
    pub(crate) async fn process_worker_ping(&self, peer_ip: SocketAddr, ping: WorkerPing<N>) {
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
                // TODO (howardwu): Limit the number of open requests we send to a peer.
                // Send an entry request to the peer.
                self.send_entry_request(peer_ip, *entry_id).await;
            }
            // Check if the entry ID exists in the pending queue for the specified peer IP.
            if !self.pending.contains_peer(*entry_id, peer_ip) {
                debug!(
                    "Worker {} - Found new entry ID '{}' from peer '{peer_ip}'",
                    self.id,
                    fmt_id(entry_id.to_string())
                );
                // Insert the entry ID into the pending queue.
                self.pending.insert(*entry_id, peer_ip);
            }
        }
    }

    /// Handles the incoming entry request.
    pub(crate) async fn process_entry_request(&self, peer_ip: SocketAddr, request: EntryRequest<N>) {
        // Check if the entry ID exists in the ready queue.
        if let Some(entry) = self.ready.get(request.entry_id) {
            // Send the entry response to the peer.
            self.send_entry_response(peer_ip, request.entry_id, entry).await;
        }
    }

    /// Handles the incoming entry response.
    pub(crate) async fn process_entry_response(&self, peer_ip: SocketAddr, response: EntryResponse<N>) -> Result<()> {
        let entry_id = response.entry_id;
        // Check if the entry ID exists in the pending queue.
        if let Some(peer_ips) = self.pending.get(entry_id) {
            // Check if the peer IP exists in the pending queue.
            if peer_ips.contains(&peer_ip) {
                // // Deserialize the entry.
                // let entry = response.entry.deserialize().await?;
                // TODO: Validate the entry.

                debug!("Worker {} - Received entry '{}' from peer '{peer_ip}'", self.id, fmt_id(entry_id.to_string()));
                // Remove the peer IP from the pending queue.
                self.pending.remove(entry_id);
                // Insert the entry into the ready queue.
                self.ready.insert(entry_id, response.entry);
            }
        }
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
        self.ready.insert(puzzle_commitment, prover_solution.into());
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
        self.ready.insert(&transaction_id, transaction.into());

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
