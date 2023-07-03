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
    helpers::{Pending, Ready, Storage, WorkerReceiver},
    Event,
    Gateway,
    TransmissionRequest,
    TransmissionResponse,
    WorkerPing,
    MAX_REQUESTS_PER_TRANSMISSION,
    MAX_WORKERS,
    WORKER_PING_INTERVAL,
};
use snarkvm::{
    console::prelude::*,
    ledger::narwhal::{Data, Transmission, TransmissionID},
    prelude::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
    },
};

use indexmap::IndexMap;
use parking_lot::Mutex;
use std::{future::Future, net::SocketAddr, sync::Arc};
use tokio::task::JoinHandle;

fn fmt_id(id: String) -> String {
    let mut formatted_id = id.chars().take(16).collect::<String>();

    if id.chars().count() > 16 {
        formatted_id.push_str("..");
    }

    formatted_id
}

#[derive(Clone)]
pub struct Worker<N: Network> {
    /// The worker ID.
    id: u8,
    /// The gateway.
    gateway: Gateway<N>,
    /// The storage.
    storage: Storage<N>,
    /// The ready queue.
    ready: Ready<N>,
    /// The pending queue.
    pending: Pending<N>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Worker<N> {
    /// Initializes a new worker instance.
    pub fn new(id: u8, gateway: Gateway<N>, storage: Storage<N>) -> Result<Self> {
        // Ensure the worker ID is valid.
        ensure!(id < MAX_WORKERS, "Invalid worker ID '{id}'");
        // Return the worker.
        Ok(Self {
            id,
            gateway,
            storage: storage.clone(),
            ready: Ready::new(storage),
            pending: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the worker instance.
    pub async fn run(&mut self, receiver: WorkerReceiver<N>) -> Result<(), Error> {
        info!("Starting worker instance {} of the memory pool...", self.id);
        // Start the worker handlers.
        self.start_handlers(receiver);
        Ok(())
    }

    /// Returns the worker ID.
    pub const fn id(&self) -> u8 {
        self.id
    }

    /// Drains the ready queue.
    pub(crate) fn drain(&self) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        self.ready.drain()
    }
}

impl<N: Network> Worker<N> {
    /// Handles the incoming ping event.
    async fn process_worker_ping(&self, peer_ip: SocketAddr, transmission_id: TransmissionID<N>) {
        // Check if the transmission ID exists in the ready queue or in storage.
        if self.ready.contains(transmission_id) || self.storage.contains(transmission_id) {
            return;
        }
        // Check if the transmission ID already exists in the ledger.
        // TODO (howardwu): Add a ledger service.

        // Retrieve the peer IPs that we have requested this transmission ID from.
        let peer_ips = self.pending.get(transmission_id).unwrap_or_default();

        // Check if the number of requests is within the limit, and we have not requested from this peer IP.
        if peer_ips.len() < MAX_REQUESTS_PER_TRANSMISSION && peer_ips.contains(&peer_ip) {
            trace!(
                "Worker {} - Found a new transmission ID '{}' from peer '{peer_ip}'",
                self.id,
                fmt_id(transmission_id.to_string())
            );
            // Insert the transmission ID into the pending queue.
            self.pending.insert(transmission_id, peer_ip);
            // TODO (howardwu): Limit the number of open requests we send to a peer.
            // Send an transmission request to the peer.
            self.send_transmission_request(peer_ip, transmission_id).await;
        }
    }

    /// Handles the incoming transmission request.
    async fn process_transmission_request(&self, peer_ip: SocketAddr, request: TransmissionRequest<N>) {
        // Check if the transmission ID exists in the ready queue.
        if let Some(transmission) = self.storage.get(request.transmission_id) {
            // Send the transmission response to the peer.
            self.send_transmission_response(peer_ip, request.transmission_id, transmission).await;
        }
    }

    /// Handles the incoming transmission response.
    async fn process_transmission_response(
        &self,
        peer_ip: SocketAddr,
        response: TransmissionResponse<N>,
    ) -> Result<()> {
        // Check if the peer IP exists in the pending queue for the given transmission ID.
        if self.pending.get(response.transmission_id).unwrap_or_default().contains(&peer_ip) {
            // Remove the transmission ID from the pending queue.
            if self.pending.remove(response.transmission_id) {
                // TODO: Validate the transmission.
                // Insert the transmission into the ready queue.
                self.ready.insert(response.transmission_id, response.transmission)?;
                debug!(
                    "Worker {} - Added transmission '{}' from peer '{peer_ip}'",
                    self.id,
                    fmt_id(response.transmission_id.to_string())
                );
            }
        }
        Ok(())
    }

    /// Handles the incoming unconfirmed solution.
    /// Note: This method assumes the incoming solution is valid and does not exist in the ledger.
    pub(crate) async fn process_unconfirmed_solution(
        &self,
        puzzle_commitment: PuzzleCommitment<N>,
        prover_solution: Data<ProverSolution<N>>,
    ) -> Result<()> {
        // Remove the puzzle commitment from the pending queue.
        self.pending.remove(puzzle_commitment);
        // Adds the prover solution to the ready queue.
        self.ready.insert(puzzle_commitment, Transmission::Solution(prover_solution))?;
        debug!("Worker {} - Added unconfirmed solution '{}'", self.id, fmt_id(puzzle_commitment.to_string()));
        Ok(())
    }

    /// Handles the incoming unconfirmed transaction.
    /// Note: This method assumes the incoming transaction is valid and does not exist in the ledger.
    pub(crate) async fn process_unconfirmed_transaction(
        &self,
        transaction_id: N::TransactionID,
        transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        // Remove the transaction from the pending queue.
        self.pending.remove(&transaction_id);
        // Adds the transaction to the ready queue.
        self.ready.insert(&transaction_id, Transmission::Transaction(transaction))?;
        debug!("Worker {} - Added unconfirmed transaction '{}'", self.id, fmt_id(transaction_id.to_string()));
        Ok(())
    }
}

impl<N: Network> Worker<N> {
    /// Starts the worker handlers.
    fn start_handlers(&self, receiver: WorkerReceiver<N>) {
        let WorkerReceiver { mut rx_worker_ping, mut rx_transmission_request, mut rx_transmission_response } = receiver;

        // Broadcast a ping event periodically.
        let self_clone = self.clone();
        self.spawn(async move {
            loop {
                // Broadcast the ping event.
                self_clone.broadcast_ping().await;
                // Wait for the next interval.
                tokio::time::sleep(std::time::Duration::from_millis(WORKER_PING_INTERVAL)).await;
            }
        });

        // Process the ping events.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, transmission_id)) = rx_worker_ping.recv().await {
                // Process the ping event.
                self_clone.process_worker_ping(peer_ip, transmission_id).await;
            }
        });

        // Process the transmission requests.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, transmission_request)) = rx_transmission_request.recv().await {
                // Process the transmission request.
                self_clone.process_transmission_request(peer_ip, transmission_request).await;
            }
        });

        // Process the transmission responses.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, transmission_response)) = rx_transmission_response.recv().await {
                // Process the transmission response.
                if let Err(e) = self_clone.process_transmission_response(peer_ip, transmission_response).await {
                    error!(
                        "Worker {} failed to process transmission response from peer '{peer_ip}': {e}",
                        self_clone.id
                    );
                }
            }
        });
    }

    /// Broadcasts a ping event.
    async fn broadcast_ping(&self) {
        // Construct the ping event.
        let ping = WorkerPing::new(self.ready.transmission_ids());
        // Broadcast the ping event.
        self.gateway.broadcast(Event::WorkerPing(ping));
    }

    /// Sends an transmission request to the specified peer.
    async fn send_transmission_request(&self, peer_ip: SocketAddr, transmission_id: TransmissionID<N>) {
        // Construct the transmission request.
        let transmission_request = TransmissionRequest::new(transmission_id);
        // Send the transmission request to the peer.
        self.gateway.send(peer_ip, Event::TransmissionRequest(transmission_request));
    }

    /// Sends an transmission response to the specified peer.
    async fn send_transmission_response(
        &self,
        peer_ip: SocketAddr,
        transmission_id: TransmissionID<N>,
        transmission: Transmission<N>,
    ) {
        // Construct the transmission response.
        let transmission_response = TransmissionResponse::new(transmission_id, transmission);
        // Send the transmission response to the peer.
        self.gateway.send(peer_ip, Event::TransmissionResponse(transmission_response));
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the worker.
    pub(crate) fn shut_down(&self) {
        trace!("Shutting down worker {}...", self.id);
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}
