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
    helpers::{assign_to_worker, init_worker_channels, Committee, PrimaryReceiver, PrimarySender, Storage},
    BatchCertified,
    BatchPropose,
    BatchSignature,
    Event,
    Gateway,
    Worker,
    MAX_BATCH_DELAY,
    MAX_EXPIRATION_TIME,
    MAX_WORKERS,
};
use snarkos_account::Account;
use snarkvm::{
    console::prelude::*,
    ledger::narwhal::{Batch, BatchCertificate, Data},
    prelude::{Field, Signature},
};

use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use std::{future::Future, net::SocketAddr, sync::Arc};
use time::OffsetDateTime;
use tokio::task::JoinHandle;

/// Returns the current UTC epoch timestamp.
fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

#[derive(Clone)]
pub struct Primary<N: Network> {
    /// The committee.
    committee: Arc<RwLock<Committee<N>>>,
    /// The gateway.
    gateway: Gateway<N>,
    /// The storage.
    storage: Storage<N>,
    /// The workers.
    workers: Arc<RwLock<Vec<Worker<N>>>>,
    /// The currently-proposed batch, along with its signatures.
    proposed_batch: Arc<RwLock<Option<(Batch<N>, IndexMap<Signature<N>, i64>)>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Primary<N> {
    /// Initializes a new primary instance.
    pub fn new(
        committee: Arc<RwLock<Committee<N>>>,
        storage: Storage<N>,
        account: Account<N>,
        dev: Option<u16>,
    ) -> Result<Self> {
        // Construct the gateway instance.
        let gateway = Gateway::new(committee.clone(), account, dev)?;
        // Return the primary instance.
        Ok(Self {
            committee,
            gateway,
            storage,
            workers: Default::default(),
            proposed_batch: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the primary instance.
    pub async fn run(&mut self, sender: PrimarySender<N>, receiver: PrimaryReceiver<N>) -> Result<()> {
        info!("Starting the primary instance of the memory pool...");

        // Set the primary sender.
        self.gateway.set_primary_sender(sender);

        // Construct a map of the worker senders.
        let mut tx_workers = IndexMap::new();

        // Initialize the workers.
        for _ in 0..MAX_WORKERS {
            // Construct the worker ID.
            let id = u8::try_from(self.workers.read().len())?;
            // Construct the worker channels.
            let (tx_worker, rx_worker) = init_worker_channels();
            // Construct the worker instance.
            let mut worker = Worker::new(id, self.gateway.clone(), self.storage.clone())?;
            // Run the worker instance.
            worker.run(rx_worker).await?;
            // Add the worker to the list of workers.
            self.workers.write().push(worker);
            // Add the worker sender to the map.
            tx_workers.insert(id, tx_worker);
        }

        // Initialize the gateway.
        self.gateway.run(tx_workers).await?;

        // Start the primary handlers.
        self.start_handlers(receiver);

        Ok(())
    }

    /// Returns the gateway.
    pub const fn gateway(&self) -> &Gateway<N> {
        &self.gateway
    }

    /// Returns the number of workers.
    pub fn num_workers(&self) -> u8 {
        u8::try_from(self.workers.read().len()).expect("Too many workers")
    }
}

impl<N: Network> Primary<N> {
    /// Proposes the batch for the current round.
    ///
    /// This method performs the following steps:
    /// 1. Drain the workers.
    /// 2. Sign the batch.
    /// 3. Set the batch in the primary.
    /// 4. Broadcast the batch to all validators for signing.
    pub fn propose_batch(&self) -> Result<()> {
        // Initialize the RNG.
        let mut rng = rand::thread_rng();

        // Initialize a map of the transmissions.
        let mut transmissions = IndexMap::new();
        // Drain the workers.
        for worker in self.workers.read().iter() {
            // TODO (howardwu): Perform one final filter against the ledger service.
            // Transition the worker to the next round, and add their transmissions to the map.
            transmissions.extend(worker.drain());
        }

        // Retrieve the private key.
        let private_key = self.gateway.account().private_key();
        // Retrieve the current round.
        let round = self.committee.read().round();
        // Compute the previous round.
        let previous_round = round.saturating_sub(1);
        // Retrieve the previous certificates.
        let previous_certificates = self.storage.get_certificates_for_round(previous_round);

        // Check if the batch is ready to be proposed.
        let mut is_ready = false;
        if previous_round == 0 {
            // Note: The primary starts at round 1, and round 0 contains no certificates, by definition.
            is_ready = true;
        } else if let Some(committee) = self.storage.get_committee_for_round(previous_round) {
            // Compute the cumulative amount of stake for the previous certificates.
            let mut stake = 0u64;
            for certificate in previous_certificates.iter() {
                stake = stake.saturating_add(committee.get_stake(certificate.to_address()));
            }
            // Check if the previous certificates have reached quorum threshold.
            if stake >= committee.quorum_threshold()? {
                is_ready = true;
            }
        }
        // If the batch is not ready to be certified, return early.
        if !is_ready {
            return Ok(());
        }

        /* Proceeding to sign & propose the batch. */

        debug!("Proposing a batch for round {round}...");

        // Sign the batch.
        let batch = Batch::new(private_key, round, transmissions, previous_certificates, &mut rng)?;
        // Retrieve the batch header.
        let header = batch.to_header()?;

        // Set the proposed batch.
        *self.proposed_batch.write() = Some((batch, Default::default()));

        // Broadcast the batch to all validators for signing.
        self.gateway.broadcast(Event::BatchPropose(BatchPropose::new(Data::Object(header))));
        Ok(())
    }

    /// Processes a batch propose from a peer.
    ///
    /// This method performs the following steps:
    /// 1. Verify the batch.
    /// 2. Sign the batch.
    /// 3. Broadcast the signature back to the validator.
    async fn process_batch_propose_from_peer(&self, peer_ip: SocketAddr, batch_propose: BatchPropose<N>) -> Result<()> {
        // Deserialize the batch header.
        let batch_header = batch_propose.batch_header.deserialize().await?;
        // Retrieve the batch ID.
        let batch_id = batch_header.batch_id();

        // TODO (howardwu): Ensure the round is within range. If not, do not sign.
        // TODO (howardwu): Ensure the address is in the committee of the specified round. If not, do not sign.
        // TODO (howardwu): Ensure the timestamp is within range. If not, do not sign.
        // TODO (howardwu): Ensure I have all of the transmissions. If not, request them before signing.
        // TODO (howardwu): Ensure I have all of the previous certificates. If not, request them before signing.
        // TODO (howardwu): Ensure the previous certificates have reached 2f+1. If not, do not sign.

        // Initialize an RNG.
        let rng = &mut rand::thread_rng();
        // Generate a timestamp.
        let timestamp = now();
        // Sign the batch ID.
        let signature = self.gateway.account().sign(&[batch_id, Field::from_u64(timestamp as u64)], rng)?;
        // Broadcast the signature back to the validator.
        self.gateway.send(peer_ip, Event::BatchSignature(BatchSignature::new(batch_id, signature, timestamp)));
        Ok(())
    }

    /// Processes a batch signature from a peer.
    ///
    /// This method performs the following steps:
    /// 1. Verify the signature, ensuring it corresponds to the proposed batch.
    /// 2. Ensure the proposed batch has not expired.
    /// 3. Store the signature.
    /// 4. Certify the batch if enough signatures have been received.
    /// 5. Broadcast the batch certificate to all validators.
    async fn process_batch_signature_from_peer(
        &self,
        peer_ip: SocketAddr,
        batch_signature: BatchSignature<N>,
    ) -> Result<()> {
        // Retrieve the batch ID and signature.
        let BatchSignature { batch_id, signature, timestamp } = batch_signature;

        // Ensure the batch ID matches the currently proposed batch.
        if Some(batch_id) != self.proposed_batch.read().as_ref().map(|(batch, _)| batch.batch_id()) {
            // Log the batch mismatch.
            match self.storage.contains_batch(batch_id) {
                true => trace!("Received a batch signature for an already certified batch from peer '{peer_ip}'"),
                false => warn!("Received a batch signature for an unknown batch from peer '{peer_ip}'"),
            }
            return Ok(());
        }
        // Retrieve the address of the peer.
        let Some(address) = self.gateway.resolver().get_address(peer_ip) else {
            warn!("Received a batch signature from a disconnected peer '{peer_ip}'");
            return Ok(());
        };
        // Ensure the address is in the committee.
        if !self.committee.read().is_committee_member(address) {
            warn!("Received a batch signature from a non-committee peer '{peer_ip}'");
            return Ok(());
        }
        // Verify the signature.
        if !signature.verify(&address, &[batch_id, Field::from_u64(timestamp as u64)]) {
            warn!("Received an invalid batch signature from peer '{peer_ip}'");
            return Ok(());
        }

        // Ensure the proposed batch has not expired, and clear the proposed batch if it has expired.
        self.check_proposed_batch_for_expiration();

        // Add the signature to the batch, and attempt to certify the batch if enough signatures have been received.
        if let Some((_, signatures)) = self.proposed_batch.write().as_mut() {
            // Add the signature to the batch.
            signatures.insert(signature, timestamp);
            debug!("Added a batch signature from peer '{peer_ip}'");
        }

        // Check if the batch is ready to be certified.
        let mut is_ready = false;
        if let Some((batch, signatures)) = self.proposed_batch.read().as_ref() {
            // Compute the cumulative amount of stake, thus far.
            let mut stake = 0u64;
            for signature in signatures.keys().chain([batch.signature()].into_iter()) {
                stake = stake.saturating_add(self.committee.read().get_stake(signature.to_address()));
            }
            // Check if the batch has reached quorum threshold.
            if stake >= self.committee.read().quorum_threshold()? {
                is_ready = true;
            }
        }
        // If the batch is not ready to be certified, return early.
        if !is_ready {
            return Ok(());
        }

        /* Proceeding to certify the batch. */

        info!("Quorum threshold reached - Preparing to certify our batch...");

        // Retrieve the batch and signatures, clearing the proposed batch.
        let (batch, signatures) = self.proposed_batch.write().take().unwrap();

        // Compute the batch header.
        let Ok(header) = batch.to_header() else {
            // TODO (howardwu): Figure out how to handle a failed header.
            error!("Failed to create a batch header");
            return Ok(());
        };

        // Create the batch certificate.
        let Ok(certificate) = BatchCertificate::new(header, signatures) else {
            // TODO (howardwu): Figure out how to handle a failed certificate.
            error!("Failed to create a batch certificate");
            return Ok(());
        };

        // Store the certified batch.
        self.storage.insert_certificate(certificate.clone())?;

        // Create a batch certified event.
        let event = BatchCertified::new(Data::Object(certificate));
        // Broadcast the certified batch to all validators.
        self.gateway.broadcast(Event::BatchCertified(event));

        // Acquire the write lock for the committee.
        let mut committee = self.committee.write();
        // Store the (now expired) committee into storage, as this round has been certified.
        self.storage.insert_committee((*committee).clone());
        // Construct the committee for the next round.
        let next_committee = (*committee).to_next_round()?;
        // Update the committee.
        *committee = next_committee;

        info!("\n\n\n\n\nOur batch for round {} has been certified!\n\n\n\n", committee.round() - 1);
        Ok(())
    }
}

impl<N: Network> Primary<N> {
    /// Starts the primary handlers.
    fn start_handlers(&self, receiver: PrimaryReceiver<N>) {
        let PrimaryReceiver {
            mut rx_batch_propose,
            mut rx_batch_signature,
            mut rx_batch_certified,
            mut rx_unconfirmed_solution,
            mut rx_unconfirmed_transaction,
        } = receiver;

        // Start the batch proposer.
        self.start_batch_proposer();

        // Process the proposed batch.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_propose)) = rx_batch_propose.recv().await {
                if let Err(e) = self_clone.process_batch_propose_from_peer(peer_ip, batch_propose).await {
                    warn!("Failed to process a batch propose from peer '{peer_ip}': {e}");
                }
            }
        });

        // Process the batch signature.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_signature)) = rx_batch_signature.recv().await {
                if let Err(e) = self_clone.process_batch_signature_from_peer(peer_ip, batch_signature).await {
                    warn!("Failed to process a batch signature from peer '{peer_ip}': {e}");
                }
            }
        });

        // Process the certified batch.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_certificate)) = rx_batch_certified.recv().await {
                // Deserialize the batch certificate.
                let Ok(batch_certificate) = batch_certificate.deserialize().await else {
                    warn!("Failed to deserialize the batch certificate from peer '{peer_ip}'");
                    continue;
                };
                // Store the batch certificate.
                if let Err(e) = self_clone.storage.insert_certificate(batch_certificate) {
                    warn!("Failed to store the batch certificate from peer '{peer_ip}' - {e}");
                    continue;
                }
            }
        });

        // Process the unconfirmed solutions.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((puzzle_commitment, prover_solution)) = rx_unconfirmed_solution.recv().await {
                // Compute the worker ID.
                let Ok(worker_id) = assign_to_worker(puzzle_commitment, self_clone.num_workers()) else {
                    error!("Unable to determine the worker ID for the unconfirmed solution");
                    continue;
                };
                // Retrieve the worker.
                let worker = self_clone.workers.read()[worker_id as usize].clone();
                // Process the unconfirmed solution.
                if let Err(e) = worker.process_unconfirmed_solution(puzzle_commitment, prover_solution).await {
                    error!("Worker {} failed process a message: {e}", worker.id());
                }
            }
        });

        // Process the unconfirmed transactions.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((transaction_id, transaction)) = rx_unconfirmed_transaction.recv().await {
                // Compute the worker ID.
                let Ok(worker_id) = assign_to_worker::<N>(&transaction_id, self_clone.num_workers()) else {
                    error!("Unable to determine the worker ID for the unconfirmed transaction");
                    continue;
                };
                // Retrieve the worker.
                let worker = self_clone.workers.read()[worker_id as usize].clone();
                // Process the unconfirmed transaction.
                if let Err(e) = worker.process_unconfirmed_transaction(transaction_id, transaction).await {
                    error!("Worker {} failed process a message: {e}", worker.id());
                }
            }
        });
    }

    /// Starts the batch proposer.
    fn start_batch_proposer(&self) {
        // Initialize the batch proposer.
        let self_clone = self.clone();
        self.spawn(async move {
            // TODO: Implement proper timeouts to propose a batch. Need to sync the primaries.
            loop {
                // Sleep briefly, but longer than if there were no batch.
                tokio::time::sleep(std::time::Duration::from_millis(MAX_BATCH_DELAY)).await;

                // Check if the proposed batch has expired, and clear it if it has expired.
                self_clone.check_proposed_batch_for_expiration();

                // If there is a proposed batch, wait for it to be certified.
                if self_clone.proposed_batch.read().is_some() {
                    continue;
                }

                // If there is no proposed batch, propose one.
                if let Err(e) = self_clone.propose_batch() {
                    error!("Failed to propose a batch: {e}");
                }
            }
        });
    }

    /// Checks if the proposed batch is expired, and clears the proposed batch if it has expired.
    fn check_proposed_batch_for_expiration(&self) {
        // Check if the proposed batch is expired.
        let mut is_expired = false;
        if let Some((batch, _)) = self.proposed_batch.read().as_ref() {
            // If the batch is expired, clear it.
            is_expired = now().saturating_sub(batch.timestamp()) > MAX_EXPIRATION_TIME;
        }
        // If the batch is expired, clear it.
        if is_expired {
            *self.proposed_batch.write() = None;
        }
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the primary.
    pub async fn shut_down(&self) {
        trace!("Shutting down the primary...");
        // Iterate through the workers.
        self.workers.read().iter().for_each(|worker| {
            // Shut down the worker.
            worker.shut_down();
        });
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
        // Close the gateway.
        self.gateway.shut_down().await;
    }
}
