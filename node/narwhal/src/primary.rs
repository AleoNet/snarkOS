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
    helpers::{assign_to_worker, init_worker_channels, Committee, Pending, PrimaryReceiver, PrimarySender, Storage},
    BatchPropose,
    BatchSignature,
    CertificateRequest,
    CertificateResponse,
    Event,
    Gateway,
    Worker,
    MAX_BATCH_DELAY,
    MAX_EXPIRATION_TIME_IN_SECS,
    MAX_TIMESTAMP_DELTA_IN_SECS,
    MAX_TRANSMISSIONS_PER_BATCH,
    MAX_WORKERS,
};
use snarkos_account::Account;
use snarkvm::{
    console::prelude::*,
    ledger::narwhal::{Batch, BatchCertificate, BatchHeader},
    prelude::{Field, Signature},
};

use futures::stream::{FuturesUnordered, StreamExt};
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use std::{collections::HashSet, future::Future, net::SocketAddr, sync::Arc};
use time::OffsetDateTime;
use tokio::{sync::oneshot, task::JoinHandle};

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
    /// The currently-proposed batch, along with its `(signature, timestamp)` entries.
    proposed_batch: Arc<RwLock<Option<(Batch<N>, IndexMap<Signature<N>, i64>)>>>,
    /// The pending certificates queue.
    pending: Pending<Field<N>>,
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
        // Insert the initial committee.
        storage.insert_committee(committee.read().clone());
        // Return the primary instance.
        Ok(Self {
            committee,
            gateway,
            storage,
            workers: Default::default(),
            proposed_batch: Default::default(),
            pending: Default::default(),
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

    /// Returns the committee.
    pub const fn committee(&self) -> &Arc<RwLock<Committee<N>>> {
        &self.committee
    }

    /// Returns the gateway.
    pub const fn gateway(&self) -> &Gateway<N> {
        &self.gateway
    }

    /// Returns the storage.
    pub const fn storage(&self) -> &Storage<N> {
        &self.storage
    }

    /// Returns the number of workers.
    pub fn num_workers(&self) -> u8 {
        u8::try_from(self.workers.read().len()).expect("Too many workers")
    }

    /// Returns the workers.
    pub const fn workers(&self) -> &Arc<RwLock<Vec<Worker<N>>>> {
        &self.workers
    }

    /// Returns the proposed batch.
    pub fn proposed_batch(&self) -> Option<Batch<N>> {
        self.proposed_batch.read().as_ref().map(|(batch, _)| batch.clone())
    }

    /// Returns the pending certificates queue.
    pub const fn pending(&self) -> &Pending<Field<N>> {
        &self.pending
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
        // If there is a batch being proposed already, return early.
        if self.proposed_batch.read().is_some() {
            // TODO (howardwu): If a proposed batch already exists:
            //  - Rebroadcast the propose batch only to nodes that have not signed.
            return Ok(());
        }

        // Retrieve the current round.
        let round = self.committee.read().round();
        // Compute the previous round.
        let previous_round = round.saturating_sub(1);
        // Retrieve the previous certificates.
        let previous_certificates = self.storage.get_certificates_for_round(previous_round);

        // Check if the batch is ready to be proposed.
        // Note: The primary starts at round 1, and round 0 contains no certificates, by definition.
        let mut is_ready = previous_round == 0;
        // If the previous round is not 0, check if the previous certificates have reached the quorum threshold.
        if previous_round > 0 {
            // Retrieve the committee for the round.
            let Some(committee) = self.storage.get_committee_for_round(previous_round) else {
                bail!("Cannot propose a batch for round {round}: the previous committee is not known yet")
            };
            // Construct a set over the authors.
            let authors = previous_certificates.iter().map(BatchCertificate::author).collect();
            // Check if the previous certificates have reached the quorum threshold.
            if committee.is_quorum_threshold_reached(&authors)? {
                is_ready = true;
            }
        }

        // If the batch is not ready to be proposed, return early.
        match is_ready {
            true => debug!("Proposing a batch for round {round}..."),
            false => return Ok(()),
        }

        /* Proceeding to sign & propose the batch. */

        // Initialize a map of the transmissions.
        let mut transmissions = IndexMap::new();
        // Drain the workers of the required number of transmissions.
        let num_transmissions_per_worker = MAX_TRANSMISSIONS_PER_BATCH / self.num_workers() as usize;
        for worker in self.workers.read().iter() {
            // TODO (howardwu): Perform one final filter against the ledger service.
            transmissions.extend(worker.take(num_transmissions_per_worker));
        }

        // Initialize the RNG.
        let rng = &mut rand::thread_rng();
        // Retrieve the private key.
        let private_key = self.gateway.account().private_key();
        // Sign the batch.
        let batch = Batch::new(private_key, round, transmissions, previous_certificates, rng)?;
        // Broadcast the batch to all validators for signing.
        self.gateway.broadcast(Event::BatchPropose(batch.to_header()?.into()));
        // Set the proposed batch.
        *self.proposed_batch.write() = Some((batch, Default::default()));
        Ok(())
    }

    /// Processes a batch propose from a peer.
    ///
    /// This method performs the following steps:
    /// 1. Verify the batch.
    /// 2. Sign the batch.
    /// 3. Broadcast the signature back to the validator.
    ///
    /// If our primary is ahead of the peer, we will not sign the batch.
    /// If our primary is behind the peer, but within GC range, we will sync up to the peer's round, and then sign the batch.
    async fn process_batch_propose_from_peer(&self, peer_ip: SocketAddr, batch_propose: BatchPropose<N>) -> Result<()> {
        let BatchPropose { round: batch_round, batch_header } = batch_propose;

        // Retrieve the committee round.
        let committee_round = self.committee.read().round();
        // TODO (howardwu): Narwhal paper implies `round`, Bullshark paper implies `round + 1`.
        // Ensure the round in the proposed batch matches the committee round.
        if committee_round > batch_round + 1 {
            bail!("Primary is on round {committee_round}, and no longer signing for round {batch_round}")
        }
        // Ensure the round declared in the batch is within GC range of the committee round.
        if committee_round + self.storage.max_gc_rounds() <= batch_round {
            bail!("Round {batch_round} is too far in the future")
        }

        // Check if the primary is still signing for the round declared in the batch.
        if let Some(round) = self.proposed_batch.read().as_ref().map(|(batch, _)| batch.round()) {
            if round != batch_round {
                bail!("Our primary is no longer signing for round {batch_round}");
            }
        }

        // TODO (howardwu): Ensure I have not signed this round for this author before. If so, do not sign.

        // Deserialize the batch header.
        let batch_header = batch_header.deserialize().await?;
        // Ensure the round matches in the batch header.
        if batch_round != batch_header.round() {
            bail!("Malicious peer - proposed round {batch_round}, but sent batch for round {}", batch_header.round());
        }
        // Ensure the batch header from the peer is valid.
        self.fetch_and_check_batch_from_peer(peer_ip, &batch_header).await?;

        /* Proceeding to sign the batch. */

        // Initialize an RNG.
        let rng = &mut rand::thread_rng();
        // Retrieve the batch ID.
        let batch_id = batch_header.batch_id();
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
    /// 1. Ensure the proposed batch has not expired.
    /// 2. Verify the signature, ensuring it corresponds to the proposed batch.
    /// 3. Store the signature.
    /// 4. Certify the batch if enough signatures have been received.
    /// 5. Broadcast the batch certificate to all validators.
    async fn process_batch_signature_from_peer(
        &self,
        peer_ip: SocketAddr,
        batch_signature: BatchSignature<N>,
    ) -> Result<()> {
        // Ensure the proposed batch has not expired, and clear the proposed batch if it has expired.
        self.check_proposed_batch_for_expiration();
        // Ensure the batch signature from the peer is valid.
        self.check_batch_signature_from_peer(peer_ip, batch_signature)?;

        // Retrieve the signature and timestamp.
        let BatchSignature { signature, timestamp, .. } = batch_signature;

        // Store the signature in the proposed batch.
        if let Some((_, signatures)) = self.proposed_batch.write().as_mut() {
            // Add the signature to the batch.
            signatures.insert(signature, timestamp);
            debug!("Added a batch signature from peer '{peer_ip}'");
        }

        // Check if the batch is ready to be certified.
        let mut is_ready = false;
        if let Some((batch, signatures)) = self.proposed_batch.read().as_ref() {
            // Construct an iterator over the addresses.
            let addresses = signatures.keys().chain([batch.signature()].into_iter()).map(Signature::to_address);
            // Check if the batch has reached the quorum threshold.
            if self.committee.read().is_quorum_threshold_reached(&addresses.collect())? {
                is_ready = true;
            }
        }
        // If the batch is not ready to be certified, return early.
        if !is_ready {
            return Ok(());
        }

        /* Proceeding to certify the batch. */

        // Retrieve the batch and signatures, clearing the proposed batch.
        let proposed_batch = self.proposed_batch.write().take();
        if let Some((batch, signatures)) = proposed_batch {
            info!("Quorum threshold reached - Preparing to certify our batch...");

            // Create the batch certificate.
            let certificate = BatchCertificate::new(batch.to_header()?, signatures)?;
            // Store the certified batch.
            self.storage.insert_certificate(certificate.clone())?;
            // Broadcast the certified batch to all validators.
            self.gateway.broadcast(Event::BatchCertified(certificate.into()));

            info!("\n\n\nOur batch for round {} has been certified!\n\n", self.committee.read().round());
            // Update the committee to the next round.
            self.update_committee_to_next_round();
        }
        Ok(())
    }

    /// Processes a batch certificate from a peer.
    ///
    /// This method performs the following steps:
    /// 1. Stores the given batch certificate, after ensuring:
    ///   - The certificate is well-formed.
    ///   - The round is within range.
    ///   - The address is in the committee of the specified round.
    ///   - We have all of the transmissions.
    ///   - We have all of the previous certificates.
    ///   - The previous certificates are valid.
    ///   - The previous certificates have reached quorum threshold.
    /// 2. Attempt to propose a batch, if there are enough certificates to reach quorum threshold for the current round.
    async fn process_batch_certificate_from_peer(
        &self,
        peer_ip: SocketAddr,
        certificate: BatchCertificate<N>,
    ) -> Result<()> {
        // Retrieve the GC round.
        let gc_round = self.storage.gc_round();
        // Retrieve the certificate round.
        let round = certificate.round();
        // If the certificate round is <= to the GC round, do not store it.
        if round <= gc_round {
            return Ok(());
        }

        // TODO (howardwu): Ensure the certificate is well-formed. If not, do not store.
        // TODO (howardwu): Ensure the address is in the committee of the specified round. If not, do not store.
        // TODO (howardwu): Ensure the previous certificates are for round-1. If not, do not store.
        // TODO (howardwu): Ensure the previous certificates have reached 2f+1. If not, do not store.

        // Ensure the primary has all of the transmissions.
        self.fetch_missing_transmissions(peer_ip, certificate.batch_header()).await?;
        // Ensure the primary has all of the previous certificates.
        self.fetch_missing_previous_certificates(peer_ip, certificate.batch_header()).await?;

        // Check if the certificate needs to be stored.
        if !self.storage.contains_certificate(certificate.certificate_id()) {
            // Store the batch certificate.
            self.storage.insert_certificate(certificate)?;
            debug!("Primary - Stored certificate for round {round} from peer '{peer_ip}'");

            // TODO (howardwu): Guard this to increment after quorum threshold is reached.
            // If the certificate's round is greater than the current committee round, update the committee.
            while self.committee.read().round() < round {
                self.update_committee_to_next_round();
            }
        }

        // // Retrieve the committee round.
        // let committee_round = self.committee.read().round();
        // // Ensure the certificate round is one less than the committee round.
        // if round + 1 != committee_round {
        //     bail!("Primary is on round {committee_round}, and received a certificate for round {round}")
        // }
        // // If there is no proposed batch, attempt to propose a batch.
        // if let Err(e) = self.propose_batch() {
        //     error!("Failed to propose a batch - {e}");
        // }
        Ok(())
    }

    /// Handles the incoming certificate request.
    fn process_certificate_request(&self, peer_ip: SocketAddr, request: CertificateRequest<N>) {
        // Attempt to retrieve the certificate.
        if let Some(certificate) = self.storage.get_certificate(request.certificate_id) {
            // Send the certificate to the peer.
            self.send_certificate_response(peer_ip, certificate);
        }
    }

    /// Handles the incoming certificate response.
    /// This method will recursively fetch any missing certificates (down to the GC round).
    async fn process_certificate_response(&self, peer_ip: SocketAddr, response: CertificateResponse<N>) -> Result<()> {
        let certificate = response.certificate;
        // Check if the peer IP exists in the pending queue for the given certificate ID.
        if self.pending.get(certificate.certificate_id()).unwrap_or_default().contains(&peer_ip) {
            // TODO: Validate the certificate.
            // Remove the certificate ID from the pending queue.
            self.pending.remove(certificate.certificate_id());
            // Store the batch certificate (recursively fetching any missing previous certificates).
            let self_clone = self.clone();
            tokio::spawn(async move {
                if let Err(e) = self_clone.process_batch_certificate_from_peer(peer_ip, certificate).await {
                    warn!("Failed to store batch certificate from peer '{peer_ip}' - {e}");
                }
            });
        }
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
            mut rx_certificate_request,
            mut rx_certificate_response,
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
                    warn!("Cannot sign proposed batch from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the batch signature.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_signature)) = rx_batch_signature.recv().await {
                if let Err(e) = self_clone.process_batch_signature_from_peer(peer_ip, batch_signature).await {
                    warn!("Cannot include a signature from peer '{peer_ip}' - {e}");
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
                if let Err(e) = self_clone.process_batch_certificate_from_peer(peer_ip, batch_certificate).await {
                    warn!("Cannot store a batch certificate from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the certificate request.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, certificate_request)) = rx_certificate_request.recv().await {
                self_clone.process_certificate_request(peer_ip, certificate_request);
            }
        });

        // Process the certificate response.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, certificate_response)) = rx_certificate_response.recv().await {
                if let Err(e) = self_clone.process_certificate_response(peer_ip, certificate_response).await {
                    warn!("Cannot process a certificate response from peer '{peer_ip}' - {e}");
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
                    error!("Worker {} failed process a message - {e}", worker.id());
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
                    error!("Worker {} failed process a message - {e}", worker.id());
                }
            }
        });
    }

    /// Starts the batch proposer.
    fn start_batch_proposer(&self) {
        // Initialize the batch proposer.
        let self_clone = self.clone();
        self.spawn(async move {
            loop {
                // Sleep briefly, but longer than if there were no batch.
                tokio::time::sleep(std::time::Duration::from_millis(MAX_BATCH_DELAY)).await;
                // Check if the proposed batch has expired, and clear it if it has expired.
                self_clone.check_proposed_batch_for_expiration();
                // If there is no proposed batch, attempt to propose a batch.
                if let Err(e) = self_clone.propose_batch() {
                    error!("Failed to propose a batch - {e}");
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
            is_expired = now().saturating_sub(batch.timestamp()) > MAX_EXPIRATION_TIME_IN_SECS;
        }
        // If the batch is expired, clear it.
        if is_expired {
            *self.proposed_batch.write() = None;
        }
    }

    /// Sanity checks the batch header from a peer.
    ///   - Ensure the round matches the committee round.
    ///   - Ensure the address is a member of the committee.
    ///   - Ensure the timestamp is within range.
    ///   - Ensure we have all of the transmissions.
    ///   - Ensure we have all of the previous certificates.
    ///   - Ensure the previous certificates are for the previous round (i.e. round - 1).
    ///   - Ensure the previous certificates have reached the quorum threshold.
    ///   - Ensure we have not already signed the batch ID.
    async fn fetch_and_check_batch_from_peer(&self, peer_ip: SocketAddr, batch_header: &BatchHeader<N>) -> Result<()> {
        // Retrieve the round.
        let batch_round = batch_header.round();

        // Ensure this batch ID is new.
        if self.storage.contains_batch(batch_header.batch_id()) {
            match ((self.committee.read().round() as i64) - batch_round as i64).abs() > 2 {
                true => bail!("Batch ID has already been processed for round {batch_round}"),
                false => return Ok(()),
            }
        }

        // If the committee cannot be found, it means this round is either too old or too new (not within GC range).
        let Some(committee) = self.storage.get_committee_for_round(batch_round) else {
            bail!("Round {batch_round} is not within our GC range")
        };
        // Ensure the author is a member of the committee.
        if !committee.is_committee_member(batch_header.author()) {
            bail!("{} is not a member of the committee", batch_header.author())
        }

        // Check the timestamp for liveness.
        self.check_timestamp_for_liveness(batch_header.timestamp())?;

        // Ensure the primary has all of the transmissions.
        self.fetch_missing_transmissions(peer_ip, batch_header).await?;
        // Ensure the primary has all of the previous certificates.
        self.fetch_missing_previous_certificates(peer_ip, batch_header).await?;

        // Retrieve the GC round.
        let gc_round = self.storage.gc_round();
        // Compute the previous round.
        let previous_round = batch_round.saturating_sub(1);

        if previous_round > 0 && previous_round > gc_round {
            // Initialize a set of the previous authors.
            let mut previous_authors = HashSet::with_capacity(batch_header.previous_certificate_ids().len());

            // Retrieve the previous certificates.
            for previous_certificate_id in batch_header.previous_certificate_ids() {
                // Retrieve the previous certificate.
                let Some(previous_certificate) = self.storage.get_certificate(*previous_certificate_id) else {
                    bail!("Missing previous certificate for a batch in round {batch_round}");
                };
                // Ensure the previous certificate is for the previous round.
                if previous_certificate.round() != previous_round {
                    bail!("Previous certificate in a batch from round {batch_round} is for the wrong round");
                }
                // Insert the author of the previous certificate.
                previous_authors.insert(previous_certificate.author());
            }

            // Ensure the previous certificates have reached the quorum threshold.
            let Some(previous_committee) = self.storage.get_committee_for_round(previous_round) else {
                bail!("Missing the committee for the previous round {previous_round}")
            };
            // Ensure the previous certificates have reached the quorum threshold.
            if !previous_committee.is_quorum_threshold_reached(&previous_authors)? {
                bail!("Previous certificates for the proposed batch did not reach quorum threshold");
            }
        }
        Ok(())
    }

    /// Sanity checks the batch signature from a peer.
    fn check_batch_signature_from_peer(&self, peer_ip: SocketAddr, batch_signature: BatchSignature<N>) -> Result<()> {
        // Retrieve the batch ID and signature.
        let BatchSignature { batch_id, signature, timestamp } = batch_signature;

        /* Check the batch ID. */

        match self.proposed_batch.read().as_ref() {
            Some((batch, _)) => {
                // Ensure the batch ID matches the currently proposed batch ID.
                if batch.batch_id() != batch_id {
                    // Log the batch mismatch.
                    match self.storage.contains_batch(batch_id) {
                        true => bail!("This batch was already certified"),
                        false => bail!("Unknown batch ID '{batch_id}'"),
                    }
                }
            }
            // Ignore the signature if there is no proposed batch currently.
            None => return Ok(()),
        };

        /* Check the signature. */

        // Retrieve the address of the peer.
        let Some(address) = self.gateway.resolver().get_address(peer_ip) else {
            bail!("Signature is from a disconnected peer")
        };
        // Ensure the address is in the committee.
        if !self.committee.read().is_committee_member(address) {
            bail!("Signature is from a non-committee peer '{address}'")
        }
        // Verify the signature.
        // Note: This check ensures the peer's address matches the signer's address.
        if !signature.verify(&address, &[batch_id, Field::from_u64(timestamp as u64)]) {
            bail!("Signature verification failed")
        }

        /* Check the timestamp. */

        self.check_timestamp_for_liveness(timestamp)?;

        Ok(())
    }

    /// Sanity checks the timestamp for liveness.
    fn check_timestamp_for_liveness(&self, timestamp: i64) -> Result<()> {
        // Ensure the timestamp is within range.
        if timestamp > (now() + MAX_TIMESTAMP_DELTA_IN_SECS) {
            bail!("Timestamp {timestamp} is too far in the future")
        }
        // TODO (howardwu): Ensure the timestamp is after the previous timestamp. (Needs Bullshark committee)
        // // Ensure the timestamp is after the previous timestamp.
        // if timestamp <= self.committee.read().previous_timestamp() {
        //     bail!("Timestamp {timestamp} for the proposed batch must be after the previous round timestamp")
        // }
        Ok(())
    }

    /// Fetches any missing transmissions for the specified batch header from the specified peer.
    async fn fetch_missing_transmissions(&self, peer_ip: SocketAddr, header: &BatchHeader<N>) -> Result<()> {
        // If the round is <= the GC round, return early.
        if header.round() <= self.storage.gc_round() {
            return Ok(());
        }

        // Initialize a list for the missing transmissions.
        let mut fetch_transmissions = FuturesUnordered::new();

        // Retrieve the number of workers.
        let num_workers = self.gateway.num_workers();
        // Iterate through the transmission IDs.
        for transmission_id in header.transmission_ids() {
            // If we do not have the transmission, request it.
            if !self.storage.contains_transmission(*transmission_id) {
                // Determine the worker ID.
                let Ok(worker_id) = assign_to_worker(*transmission_id, num_workers) else {
                    bail!("Unable to assign transmission ID '{transmission_id}' to a worker")
                };
                // Initialize a oneshot channel.
                let (callback_sender, callback_receiver) = oneshot::channel();
                // Retrieve the worker.
                match self.workers.read().get(worker_id as usize) {
                    Some(worker) => {
                        // Send the transmission ID to the worker.
                        worker.process_transmission_id(peer_ip, *transmission_id, Some(callback_sender));
                        // Push the callback onto the list.
                        fetch_transmissions.push(callback_receiver);
                    }
                    None => bail!("Unable to find worker {worker_id}"),
                }
            }
        }

        // Wait for all of the transmissions to be fetched.
        while let Some(result) = fetch_transmissions.next().await {
            if let Err(e) = result {
                bail!("Unable to fetch transmission: {e}")
            }
        }
        // Return after receiving all of the transmissions.
        Ok(())
    }

    /// Fetches any missing previous certificates for the specified batch header from the specified peer.
    async fn fetch_missing_previous_certificates(&self, peer_ip: SocketAddr, header: &BatchHeader<N>) -> Result<()> {
        // If the previous round is 0, or is <= the GC round, return early.
        if header.round() == 1 || header.round() <= self.storage.gc_round() + 1 {
            return Ok(());
        }

        // Initialize a list for the missing previous certificates.
        let mut fetch_certificates = FuturesUnordered::new();

        // Iterate through the previous certificate IDs.
        for certificate_id in header.previous_certificate_ids() {
            // Ensure that we have not requested this certificate from this peer before.
            if self.pending.get(*certificate_id).unwrap_or_default().contains(&peer_ip) {
                continue;
            }

            // TODO (howardwu): This conditional can be simplified, however the logic here is still unstable.
            //  As such, only update this after we have finished implementing the 'syncing' logic.
            // If we do not have the previous certificate, request it.
            if self.pending.contains(*certificate_id) && !self.storage.contains_certificate(*certificate_id) {
                // Initialize a oneshot channel.
                let (callback_sender, callback_receiver) = oneshot::channel();
                // Insert the certificate ID into the pending queue.
                self.pending.insert(*certificate_id, peer_ip, Some(callback_sender));
                // Push the callback onto the list.
                fetch_certificates.push(callback_receiver);
            } else if !self.pending.contains(*certificate_id) && !self.storage.contains_certificate(*certificate_id) {
                trace!("Primary - Found a new certificate ID for round {} from peer '{peer_ip}'", header.round());

                // Initialize a oneshot channel.
                let (callback_sender, callback_receiver) = oneshot::channel();
                // Insert the certificate ID into the pending queue.
                self.pending.insert(*certificate_id, peer_ip, Some(callback_sender));
                // TODO (howardwu): Limit the number of open requests we send to a peer.
                // Send an certificate request to the peer.
                self.send_certificate_request(peer_ip, *certificate_id);
                // Push the callback onto the list.
                fetch_certificates.push(callback_receiver);
            }
        }

        // Wait for all of the certificates to be fetched.
        while let Some(result) = fetch_certificates.next().await {
            if let Err(e) = result {
                bail!("Unable to fetch certificate: {e}")
            }
        }
        // Return after receiving all of the certificates.
        Ok(())
    }

    /// Sends an certificate request to the specified peer.
    fn send_certificate_request(&self, peer_ip: SocketAddr, certificate_id: Field<N>) {
        // Send the certificate request to the peer.
        self.gateway.send(peer_ip, Event::CertificateRequest(certificate_id.into()));
    }

    /// Sends an certificate response to the specified peer.
    fn send_certificate_response(&self, peer_ip: SocketAddr, certificate: BatchCertificate<N>) {
        // Send the certificate response to the peer.
        self.gateway.send(peer_ip, Event::CertificateResponse(certificate.into()));
    }

    /// Updates the committee to the next round, returning the next round number.
    fn update_committee_to_next_round(&self) -> u64 {
        // TODO (howardwu): Move this logic to Bullshark, as:
        //  - We need to know which members (and stake) to add, update, and remove.
        // Acquire the write lock for the committee.
        let mut committee = self.committee.write();
        // Construct the committee for the next round.
        let next_committee = (*committee).to_next_round();
        // Store the next committee into storage.
        self.storage.insert_committee(next_committee.clone());
        // Update the committee.
        *committee = next_committee;
        // Clear the proposed batch.
        *self.proposed_batch.write() = None;
        // Log the updated round.
        info!("Starting round {}...", committee.round());
        // Return the next round number.
        committee.round()
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
