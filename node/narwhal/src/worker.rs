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
    event::{Event, TransmissionRequest, TransmissionResponse},
    helpers::{fmt_id, Pending, Ready, Storage, WorkerReceiver},
    Gateway,
    Ledger,
    ProposedBatch,
    MAX_BATCH_DELAY,
    MAX_TRANSMISSIONS_PER_BATCH,
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

use indexmap::IndexSet;
use parking_lot::Mutex;
use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{sync::oneshot, task::JoinHandle, time::timeout};

#[derive(Clone)]
pub struct Worker<N: Network> {
    /// The worker ID.
    id: u8,
    /// The gateway.
    gateway: Gateway<N>,
    /// The storage.
    storage: Storage<N>,
    /// The ledger service.
    ledger: Arc<Ledger<N>>,
    /// The proposed batch.
    proposed_batch: Arc<ProposedBatch<N>>,
    /// The ready queue.
    ready: Ready<N>,
    /// The pending transmissions queue.
    pending: Arc<Pending<TransmissionID<N>, Transmission<N>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Worker<N> {
    /// Initializes a new worker instance.
    pub fn new(
        id: u8,
        gateway: Gateway<N>,
        storage: Storage<N>,
        ledger: Arc<Ledger<N>>,
        proposed_batch: Arc<ProposedBatch<N>>,
    ) -> Result<Self> {
        // Ensure the worker ID is valid.
        ensure!(id < MAX_WORKERS, "Invalid worker ID '{id}'");
        // Return the worker.
        Ok(Self {
            id,
            gateway,
            storage: storage.clone(),
            ledger,
            proposed_batch,
            ready: Ready::new(storage),
            pending: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the worker instance.
    pub fn run(&self, receiver: WorkerReceiver<N>) {
        info!("Starting worker instance {} of the memory pool...", self.id);
        // Start the worker handlers.
        self.start_handlers(receiver);
    }

    /// Returns the worker ID.
    pub const fn id(&self) -> u8 {
        self.id
    }

    /// Returns `true` if the transmission ID exists in the ready queue, proposed batch, storage, or ledger.
    pub fn contains_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        let transmission_id = transmission_id.into();
        // Check if the transmission ID exists in the ready queue, proposed batch, storage, or ledger.
        self.ready.contains(transmission_id)
            || self.proposed_batch.read().as_ref().map_or(false, |p| p.contains_transmission(transmission_id))
            || self.storage.contains_transmission(transmission_id)
            || self.ledger.contains_transmission(&transmission_id).unwrap_or(false)
    }

    /// Returns the transmission if it exists in the ready queue, proposed batch, storage, or ledger.
    pub fn get_transmission(&self, transmission_id: TransmissionID<N>) -> Option<Transmission<N>> {
        // Check if the transmission ID exists in the ready queue.
        if let Some(transmission) = self.ready.get(transmission_id) {
            return Some(transmission);
        }
        // Check if the transmission ID exists in storage.
        if let Some(transmission) = self.storage.get_transmission(transmission_id) {
            return Some(transmission);
        }
        // Check if the transmission ID exists in the proposed batch.
        if let Some(transmission) =
            self.proposed_batch.read().as_ref().and_then(|p| p.get_transmission(transmission_id))
        {
            return Some(transmission.clone());
        }
        // Check if the transmission ID already exists in the ledger.
        if let Some(transmission) = self.ledger.get_transmission(&transmission_id).unwrap_or(None) {
            return Some(transmission);
        }
        None
    }

    /// Returns the transmissions if it exists in the worker, or requests it from the specified peer.
    pub async fn get_or_fetch_transmission(
        &self,
        peer_ip: SocketAddr,
        transmission_id: TransmissionID<N>,
    ) -> Result<(TransmissionID<N>, Transmission<N>)> {
        // Attempt to get the transmission from the worker.
        if let Some(transmission) = self.get_transmission(transmission_id) {
            return Ok((transmission_id, transmission));
        }
        // Send a transmission request to the peer.
        let (candidate_id, transmission) = self.send_transmission_request(peer_ip, transmission_id).await?;
        // Ensure the transmission ID matches.
        ensure!(candidate_id == transmission_id, "Invalid transmission ID");
        // Return the transmission.
        Ok((transmission_id, transmission))
    }

    /// Removes the specified number of transmissions from the ready queue, and returns them.
    pub(crate) fn take_candidates(
        &self,
        num_transmissions: usize,
    ) -> impl Iterator<Item = (TransmissionID<N>, Transmission<N>)> {
        // Acquire the proposed batch read lock.
        let proposed_batch = self.proposed_batch.read();
        // Retain the transmissions that are not in the storage or ledger.
        self.ready.retain(|id, _| {
            !self.storage.contains_transmission(*id)
                && !proposed_batch.as_ref().map_or(false, |p| p.contains_transmission(*id))
                && !self.ledger.contains_transmission(id).unwrap_or(false)
        });
        // Remove the specified number of transmissions from the ready queue.
        self.ready.take(num_transmissions).into_iter()
    }

    /// Reinserts the specified transmission into the ready queue.
    pub(crate) fn reinsert(&self, transmission_id: TransmissionID<N>, transmission: Transmission<N>) -> bool {
        // Check if the transmission ID exists.
        if !self.contains_transmission(transmission_id) {
            // Insert the transmission into the ready queue.
            return self.ready.insert(transmission_id, transmission);
        }
        false
    }
}

impl<N: Network> Worker<N> {
    /// Handles the incoming transmission ID from a worker ping event.
    async fn process_transmission_id_from_ping(
        &self,
        peer_ip: SocketAddr,
        transmission_id: TransmissionID<N>,
    ) -> Result<()> {
        // Check if the transmission ID exists.
        if self.contains_transmission(transmission_id) {
            return Ok(());
        }
        // If the ready queue is full, then skip this transmission.
        // Note: We must prioritize the unconfirmed solutions and unconfirmed transactions, not transmissions.
        if self.ready.len() > MAX_TRANSMISSIONS_PER_BATCH {
            return Ok(());
        }
        trace!("Worker {} - Found a new transmission ID '{}' from peer '{peer_ip}'", self.id, fmt_id(transmission_id));
        // Send an transmission request to the peer.
        let (candidate_id, transmission) = self.send_transmission_request(peer_ip, transmission_id).await?;
        // Ensure the transmission ID matches.
        ensure!(candidate_id == transmission_id, "Invalid transmission ID");
        // Insert the transmission into the ready queue.
        self.process_transmission_from_peer(peer_ip, transmission_id, transmission);
        Ok(())
    }

    /// Handles the incoming transmission from a peer.
    pub(crate) fn process_transmission_from_peer(
        &self,
        peer_ip: SocketAddr,
        transmission_id: TransmissionID<N>,
        transmission: Transmission<N>,
    ) {
        // Check if the transmission ID exists.
        if !self.contains_transmission(transmission_id) {
            // Insert the transmission into the ready queue.
            self.ready.insert(transmission_id, transmission);
            trace!("Worker {} - Added transmission '{}' from peer '{peer_ip}'", self.id, fmt_id(transmission_id));
        }
    }

    /// Handles the incoming unconfirmed solution.
    /// Note: This method assumes the incoming solution is valid and does not exist in the ledger.
    pub(crate) fn process_unconfirmed_solution(
        &self,
        puzzle_commitment: PuzzleCommitment<N>,
        prover_solution: Data<ProverSolution<N>>,
    ) {
        // Construct the transmission.
        let transmission = Transmission::Solution(prover_solution);
        // Remove the puzzle commitment from the pending queue.
        self.pending.remove(puzzle_commitment, Some(transmission.clone()));
        // Check if the solution exists.
        if !self.contains_transmission(puzzle_commitment) {
            // Adds the prover solution to the ready queue.
            self.ready.insert(puzzle_commitment, transmission);
            trace!("Worker {} - Added unconfirmed solution '{}'", self.id, fmt_id(puzzle_commitment));
        }
    }

    /// Handles the incoming unconfirmed transaction.
    /// Note: This method assumes the incoming transaction is valid and does not exist in the ledger.
    pub(crate) fn process_unconfirmed_transaction(
        &self,
        transaction_id: N::TransactionID,
        transaction: Data<Transaction<N>>,
    ) {
        // Construct the transmission.
        let transmission = Transmission::Transaction(transaction);
        // Remove the transaction from the pending queue.
        self.pending.remove(&transaction_id, Some(transmission.clone()));
        // Check if the transaction ID exists.
        if !self.contains_transmission(&transaction_id) {
            // Adds the transaction to the ready queue.
            self.ready.insert(&transaction_id, transmission);
            trace!("Worker {} - Added unconfirmed transaction '{}'", self.id, fmt_id(transaction_id));
        }
    }
}

impl<N: Network> Worker<N> {
    /// Starts the worker handlers.
    fn start_handlers(&self, receiver: WorkerReceiver<N>) {
        let WorkerReceiver { mut rx_worker_ping, mut rx_transmission_request, mut rx_transmission_response } = receiver;

        // Broadcast a ping event periodically.
        let self_ = self.clone();
        self.spawn(async move {
            loop {
                // Broadcast the ping event.
                self_.broadcast_ping();
                // Wait for the next interval.
                tokio::time::sleep(Duration::from_millis(WORKER_PING_INTERVAL)).await;
            }
        });

        // Process the ping events.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, transmission_id)) = rx_worker_ping.recv().await {
                if let Err(e) = self_.process_transmission_id_from_ping(peer_ip, transmission_id).await {
                    warn!(
                        "Worker {} failed to fetch missing transmission '{}' from peer '{peer_ip}': {e}",
                        self_.id,
                        fmt_id(transmission_id)
                    );
                }
            }
        });

        // Process the transmission requests.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, transmission_request)) = rx_transmission_request.recv().await {
                self_.send_transmission_response(peer_ip, transmission_request);
            }
        });

        // Process the transmission responses.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, transmission_response)) = rx_transmission_response.recv().await {
                // Process the transmission response.
                self_.finish_transmission_request(peer_ip, transmission_response);
            }
        });
    }

    /// Broadcasts a ping event.
    fn broadcast_ping(&self) {
        // Broadcast the ping event.
        self.gateway.broadcast(Event::WorkerPing(
            self.ready.transmission_ids().into_iter().take(MAX_TRANSMISSIONS_PER_BATCH).collect::<IndexSet<_>>().into(),
        ));
    }

    /// Sends an transmission request to the specified peer.
    async fn send_transmission_request(
        &self,
        peer_ip: SocketAddr,
        transmission_id: TransmissionID<N>,
    ) -> Result<(TransmissionID<N>, Transmission<N>)> {
        // Initialize a oneshot channel.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Insert the transmission ID into the pending queue.
        self.pending.insert(transmission_id, peer_ip, Some(callback_sender));
        // Send the transmission request to the peer.
        self.gateway.send(peer_ip, Event::TransmissionRequest(transmission_id.into()));
        // Wait for the transmission to be fetched.
        match timeout(Duration::from_millis(MAX_BATCH_DELAY), callback_receiver).await {
            // If the transmission was fetched, return it.
            Ok(result) => Ok((transmission_id, result?)),
            // If the transmission was not fetched, return an error.
            Err(e) => bail!("Unable to fetch transmission - (timeout) {e}"),
        }
    }

    /// Handles the incoming transmission response.
    /// This method ensures the transmission response is well-formed and matches the transmission ID.
    fn finish_transmission_request(&self, peer_ip: SocketAddr, response: TransmissionResponse<N>) {
        let TransmissionResponse { transmission_id, transmission } = response;
        // Check if the peer IP exists in the pending queue for the given transmission ID.
        let exists = self.pending.get(transmission_id).unwrap_or_default().contains(&peer_ip);
        // If the peer IP exists, finish the pending request.
        if exists {
            // TODO: Validate the transmission.
            // TODO (howardwu): Deserialize the transmission, and ensure it matches the transmission ID.
            //  Note: This is difficult for testing and example purposes, since those transmissions are fake.
            // Remove the transmission ID from the pending queue.
            self.pending.remove(transmission_id, Some(transmission));
        }
    }

    /// Sends the requested transmission to the specified peer.
    fn send_transmission_response(&self, peer_ip: SocketAddr, request: TransmissionRequest<N>) {
        let TransmissionRequest { transmission_id } = request;
        // Attempt to retrieve the transmission.
        if let Some(transmission) = self.get_transmission(transmission_id) {
            // Send the transmission response to the peer.
            self.gateway.send(peer_ip, Event::TransmissionResponse((transmission_id, transmission).into()));
        }
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

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::{helpers::storage::prop_tests::StorageInput, prop_tests::GatewayInput};
    use snarkos_node_narwhal_ledger_service::MockLedgerService;

    use test_strategy::{proptest, Arbitrary};

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    #[derive(Arbitrary, Debug, Clone)]
    pub struct WorkerInput {
        pub id: u8,
        #[filter(GatewayInput::is_valid)]
        pub gateway: GatewayInput,
        pub storage: StorageInput,
    }

    impl WorkerInput {
        fn to_worker(&self) -> Result<Worker<CurrentNetwork>> {
            Worker::new(
                self.id,
                self.gateway.to_gateway(),
                self.storage.to_storage(),
                Arc::new(Box::new(MockLedgerService::new())),
                Default::default(),
            )
        }

        fn is_valid(&self) -> bool {
            self.id < MAX_WORKERS
        }
    }

    #[proptest]
    fn worker_initialization(input: WorkerInput) {
        match input.to_worker() {
            Ok(worker) => {
                assert!(input.is_valid());
                assert_eq!(worker.id(), input.id);
            }
            Err(e) => {
                assert!(!input.is_valid());
                assert_eq!(e.to_string().as_str(), format!("Invalid worker ID '{}'", input.id));
            }
        }
    }
}
