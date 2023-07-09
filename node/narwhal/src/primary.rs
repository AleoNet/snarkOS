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
    helpers::{
        assign_to_worker,
        init_worker_channels,
        now,
        Committee,
        Pending,
        PrimaryReceiver,
        PrimarySender,
        Proposal,
        Storage,
    },
    BatchPropose,
    BatchSignature,
    CertificateRequest,
    CertificateResponse,
    Event,
    Gateway,
    Worker,
    MAX_BATCH_DELAY,
    MAX_TRANSMISSIONS_PER_BATCH,
    MAX_WORKERS,
};
use snarkos_account::Account;
use snarkvm::{
    console::prelude::*,
    ledger::narwhal::{Batch, BatchCertificate, BatchHeader, Transmission, TransmissionID},
    prelude::Field,
};

use async_recursion::async_recursion;
use futures::stream::{FuturesUnordered, StreamExt};
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::{sync::oneshot, task::JoinHandle, time::timeout};

#[derive(Clone)]
pub struct Primary<N: Network> {
    /// The committee.
    committee: Arc<RwLock<Committee<N>>>,
    /// The gateway.
    gateway: Gateway<N>,
    /// The storage.
    storage: Storage<N>,
    /// The workers.
    workers: Arc<Vec<Worker<N>>>,
    /// The batch proposal, if the primary is currently proposing a batch.
    proposed_batch: Arc<RwLock<Option<Proposal<N>>>>,
    /// The pending certificates queue.
    pending: Pending<Field<N>, BatchCertificate<N>>,
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
        // Construct a map for the workers.
        let mut workers = Vec::new();
        // Initialize the workers.
        for id in 0..MAX_WORKERS {
            // Construct the worker channels.
            let (tx_worker, rx_worker) = init_worker_channels();
            // Construct the worker instance.
            let worker = Worker::new(id, self.gateway.clone(), self.storage.clone())?;
            // Run the worker instance.
            worker.run(rx_worker).await?;
            // Add the worker to the list of workers.
            workers.push(worker);
            // Add the worker sender to the map.
            tx_workers.insert(id, tx_worker);
        }
        // Set the workers.
        self.workers = Arc::new(workers);

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
        u8::try_from(self.workers.len()).expect("Too many workers")
    }

    /// Returns the workers.
    pub const fn workers(&self) -> &Arc<Vec<Worker<N>>> {
        &self.workers
    }

    /// Returns the batch proposal of our primary, if one currently exists.
    pub fn batch_proposal(&self) -> &Arc<RwLock<Option<Proposal<N>>>> {
        &self.proposed_batch
    }
}

impl<N: Network> Primary<N> {
    /// Proposes the batch for the current round.
    ///
    /// This method performs the following steps:
    /// 1. Drain the workers.
    /// 2. Sign the batch.
    /// 3. Set the batch proposal in the primary.
    /// 4. Broadcast the batch header to all validators for signing.
    pub fn propose_batch(&self) -> Result<()> {
        // Check if the proposed batch has expired, and clear it if it has expired.
        self.check_proposed_batch_for_expiration()?;
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
            let Some(committee) = self.storage.get_committee(previous_round) else {
                bail!("Cannot propose a batch for round {round}: the previous committee is not known yet")
            };
            // Construct a set over the authors.
            let authors = previous_certificates.iter().map(BatchCertificate::author).collect();
            // Check if the previous certificates have reached the quorum threshold.
            if committee.is_quorum_threshold_reached(&authors) {
                is_ready = true;
            }
        }
        // If the batch is not ready to be proposed, return early.
        if !is_ready {
            return Ok(());
        }

        // Initialize a map of the transmissions.
        let mut transmissions = IndexMap::new();
        // Drain the workers of the required number of transmissions.
        let num_transmissions_per_worker = MAX_TRANSMISSIONS_PER_BATCH / self.num_workers() as usize;
        for worker in self.workers.iter() {
            // TODO (howardwu): Perform one final filter against the ledger service.
            transmissions.extend(worker.take(num_transmissions_per_worker));
        }
        // Determine if there are transmissions to propose.
        let has_transmissions = !transmissions.is_empty();

        // If the batch is not ready to be proposed, return early.
        match has_transmissions {
            true => info!("Proposing a batch with {} transmissions for round {round}...", transmissions.len()),
            false => return Ok(()),
        }

        /* Proceeding to sign & propose the batch. */

        // Initialize the RNG.
        let rng = &mut rand::thread_rng();
        // Retrieve the private key.
        let private_key = self.gateway.account().private_key();
        // Sign the batch.
        let batch = Batch::new(private_key, round, transmissions, previous_certificates, rng)?;
        // Construct the batch header.
        let batch_header = batch.to_header()?;
        // Construct the proposal.
        let proposal = Proposal::new(self.committee.read().clone(), batch)?;
        // Broadcast the batch to all validators for signing.
        self.gateway.broadcast(Event::BatchPropose(batch_header.into()));
        // Set the proposed batch.
        *self.proposed_batch.write() = Some(proposal);
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

        // Ensure the batch is for the current round.
        self.ensure_is_signing_round(batch_round)?;

        // TODO (howardwu): Ensure I have not signed this round for this author before. If so, do not sign.

        // Deserialize the batch header.
        let batch_header = batch_header.deserialize().await?;
        // Ensure the round matches in the batch header.
        if batch_round != batch_header.round() {
            bail!("Malicious peer - proposed round {batch_round}, but sent batch for round {}", batch_header.round());
        }

        // TODO (howardwu): Include fetching from the peer's proposed batch, to fix this fetch that times out.
        // // Ensure the primary has all of the transmissions.
        // let transmissions = self.fetch_missing_transmissions(peer_ip, &batch_header).await?;
        // // TODO (howardwu): Add the missing transmissions into the workers.
        // // Ensure the batch header from the peer is valid.
        // let missing_transmissions = self.storage.check_batch_header(&batch_header, transmissions)?;

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
        debug!("Signed a batch for round {batch_round} from peer '{peer_ip}'");
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
        self.check_proposed_batch_for_expiration()?;

        // Retrieve the signature and timestamp.
        let BatchSignature { batch_id, signature, timestamp } = batch_signature;

        // Acquire the write lock.
        let mut proposed_batch = self.proposed_batch.write();

        // Add the signature to the batch, and determine if the batch is ready to be certified.
        let is_ready = match proposed_batch.as_mut() {
            Some(proposal) => {
                // Ensure the batch ID matches the currently proposed batch ID.
                if proposal.batch_id() != batch_id {
                    match self.storage.contains_batch(batch_id) {
                        true => bail!("This batch was already certified"),
                        false => bail!("Unknown batch ID '{batch_id}'"),
                    }
                }
                // Retrieve the address of the peer.
                let Some(signer) = self.gateway.resolver().get_address(peer_ip) else {
                    bail!("Signature is from a disconnected peer")
                };
                // Add the signature to the batch.
                proposal.add_signature(signer, signature, timestamp)?;
                info!("Added a batch signature from peer '{peer_ip}'");
                // Check if the batch is ready to be certified.
                proposal.is_quorum_threshold_reached()
            }
            None => false,
        };

        // If the batch is not ready to be certified, return early.
        if !is_ready {
            return Ok(());
        }

        /* Proceeding to certify the batch. */

        // Retrieve the batch proposal, clearing the proposed batch.
        let proposal = proposed_batch.take();
        drop(proposed_batch);

        // Certify the batch.
        if let Some(proposal) = proposal {
            info!("Quorum threshold reached - Preparing to certify our batch...");

            // TODO (howardwu): If any method below fails, we need to return the transmissions back to the ready queue.
            // Create the batch certificate and transmissions.
            let (certificate, transmissions) = proposal.into_certificate()?;
            // Store the certified batch.
            self.storage.insert_certificate(certificate.clone(), transmissions)?;
            // Broadcast the certified batch to all validators.
            self.gateway.broadcast(Event::BatchCertified(certificate.clone().into()));

            info!(
                "\n\nOur batch with {} transmissions for round {} was certified!\n",
                certificate.transmission_ids().len(),
                certificate.round()
            );
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
        self.sync_with_peer(peer_ip, certificate).await
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
        let self_ = self.clone();
        self.spawn(async move {
            loop {
                // Sleep briefly, but longer than if there were no batch.
                tokio::time::sleep(Duration::from_millis(MAX_BATCH_DELAY)).await;
                // If there is no proposed batch, attempt to propose a batch.
                if let Err(e) = self_.propose_batch() {
                    error!("Failed to propose a batch - {e}");
                }
            }
        });

        // Process the proposed batch.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_propose)) = rx_batch_propose.recv().await {
                if let Err(e) = self_.process_batch_propose_from_peer(peer_ip, batch_propose).await {
                    warn!("Cannot sign proposed batch from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the batch signature.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_signature)) = rx_batch_signature.recv().await {
                if let Err(e) = self_.process_batch_signature_from_peer(peer_ip, batch_signature).await {
                    warn!("Cannot include a signature from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the certified batch.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_certificate)) = rx_batch_certified.recv().await {
                // Deserialize the batch certificate.
                let Ok(batch_certificate) = batch_certificate.deserialize().await else {
                    warn!("Failed to deserialize the batch certificate from peer '{peer_ip}'");
                    continue;
                };
                if let Err(e) = self_.process_batch_certificate_from_peer(peer_ip, batch_certificate).await {
                    warn!("Cannot store a batch certificate from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the certificate request.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, certificate_request)) = rx_certificate_request.recv().await {
                self_.send_certificate_response(peer_ip, certificate_request);
            }
        });

        // Process the certificate response.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, certificate_response)) = rx_certificate_response.recv().await {
                self_.finish_certificate_request(peer_ip, certificate_response)
            }
        });

        // Process the unconfirmed solutions.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((puzzle_commitment, prover_solution)) = rx_unconfirmed_solution.recv().await {
                // Compute the worker ID.
                let Ok(worker_id) = assign_to_worker(puzzle_commitment, self_.num_workers()) else {
                    error!("Unable to determine the worker ID for the unconfirmed solution");
                    continue;
                };
                // Process the unconfirmed solution.
                self_.workers[worker_id as usize].process_unconfirmed_solution(puzzle_commitment, prover_solution)
            }
        });

        // Process the unconfirmed transactions.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((transaction_id, transaction)) = rx_unconfirmed_transaction.recv().await {
                // Compute the worker ID.
                let Ok(worker_id) = assign_to_worker::<N>(&transaction_id, self_.num_workers()) else {
                    error!("Unable to determine the worker ID for the unconfirmed transaction");
                    continue;
                };
                // Process the unconfirmed transaction.
                self_.workers[worker_id as usize].process_unconfirmed_transaction(transaction_id, transaction)
            }
        });
    }

    /// Checks if the proposed batch is expired, and clears the proposed batch if it has expired.
    fn check_proposed_batch_for_expiration(&self) -> Result<()> {
        // Check if the proposed batch is expired.
        let is_expired = self.proposed_batch.read().as_ref().map_or(false, Proposal::is_expired);
        // If the batch is expired, clear it.
        if is_expired {
            // Reset the proposed batch.
            if let Some(proposal) = self.proposed_batch.write().take() {
                // Retrieve the number of workers.
                let num_workers = self.num_workers();
                // Re-insert the transmissions into the workers.
                for (transmission_id, transmission) in proposal.transmissions() {
                    // Determine the worker ID.
                    let Ok(worker_id) = assign_to_worker(*transmission_id, num_workers) else {
                        bail!("Unable to assign transmission ID '{transmission_id}' to a worker")
                    };
                    // Retrieve the worker.
                    match self.workers.get(worker_id as usize) {
                        // Re-insert the transmission into the worker.
                        Some(worker) => worker.reinsert(*transmission_id, transmission.clone()),
                        None => bail!("Unable to find worker {worker_id}"),
                    };
                }
            }

            // TODO (howardwu): Guard this to increment after quorum threshold is reached.
            // TODO (howardwu): After bullshark is implemented, we must use Aleo blocks to guide us to `tip-50` to know the committee.
            // Initialize a tracker to increment the round.
            let mut current_round = self.committee.read().round();
            // Check if there are certificates for the next round.
            while !self.storage.get_certificates_for_round(current_round + 1).is_empty() {
                // If there are certificates for the next round, increment the round.
                self.update_committee_to_next_round();
                // Increment the current round.
                current_round += 1;
            }
        }
        Ok(())
    }

    /// Ensures the primary is signing for the specified batch round.
    fn ensure_is_signing_round(&self, batch_round: u64) -> Result<()> {
        // Retrieve the committee round.
        let committee_round = self.committee.read().round();
        // Ensure the batch round is within GC range of the committee round.
        if committee_round + self.storage.max_gc_rounds() <= batch_round {
            bail!("Round {batch_round} is too far in the future")
        }
        // Ensure the batch round is at or one before the committee round.
        // Intuition: Our primary has moved on to the next round, but has not necessarily started proposing,
        // so we can still sign for the previous round. If we have started proposing, the next check will fail.
        if committee_round > batch_round + 1 {
            bail!("Primary is on round {committee_round}, and no longer signing for round {batch_round}")
        }
        // Check if the primary is still signing for the batch round.
        if let Some(signing_round) = self.proposed_batch.read().as_ref().map(|proposal| proposal.round()) {
            if signing_round > batch_round {
                bail!("Our primary at round {signing_round} is no longer signing for round {batch_round}")
            }
        }
        Ok(())
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
    #[async_recursion]
    async fn sync_with_peer(&self, peer_ip: SocketAddr, certificate: BatchCertificate<N>) -> Result<()> {
        // Retrieve the batch header.
        let batch_header = certificate.batch_header();
        // Retrieve the batch round.
        let batch_round = batch_header.round();
        // Retrieve the GC round.
        let gc_round = self.storage.gc_round();

        // If the certificate round is outdated, do not store it.
        if batch_round <= gc_round {
            return Ok(());
        }
        // If the certificate already exists in storage, return early.
        if self.storage.contains_certificate(certificate.certificate_id()) {
            return Ok(());
        }

        // // Check if
        // self.storage.get_certificates_for_round(round).into_iter().chain([certificate.clone()].into_iter());

        // Check if our primary should move to the next round.
        let is_behind_schedule = batch_round > self.committee.read().round(); // TODO: Check if threshold is reached.
        // Check if our primary is far behind the peer.
        let is_out_of_range = batch_round > gc_round + self.storage.max_gc_rounds();
        // If our primary is far behind the peer, update our committee to the batch round.
        if is_behind_schedule || is_out_of_range {
            // TODO (howardwu): Guard this to increment after quorum threshold is reached.
            // TODO (howardwu): After bullshark is implemented, we must use Aleo blocks to guide us to `tip-50` to know the committee.
            // If the batch round is greater than the current committee round, update the committee.
            while self.committee.read().round() < batch_round {
                self.update_committee_to_next_round();
            }
        }

        // // Ensure this batch does not contain already committed transmissions from past rounds.
        // if batch_header.transmission_ids().iter().any(|id| self.storage.contains_transmission(*id)) {
        //     bail!("Batch contains already transmissions from past rounds");
        // }
        // Ensure this batch does not contain already committed transmissions in the ledger.
        // TODO: Add a ledger service.

        // Ensure the primary has all of the previous certificates.
        let missing_certificates = self.fetch_missing_previous_certificates(peer_ip, batch_header).await?;
        // Iterate through the missing certificates.
        for batch_certificate in missing_certificates {
            // Store the batch certificate (recursively fetching any missing previous certificates).
            self.sync_with_peer(peer_ip, batch_certificate).await?;
        }

        // Ensure the primary has all of the transmissions.
        let missing_transmissions = self.fetch_missing_transmissions(peer_ip, batch_header).await?;
        // Check if the certificate needs to be stored.
        if !self.storage.contains_certificate(certificate.certificate_id()) {
            // Store the batch certificate.
            self.storage.insert_certificate(certificate, missing_transmissions)?;
            debug!("Stored certificate for round {batch_round} from peer '{peer_ip}'");
        }
        Ok(())
    }

    /// Fetches any missing transmissions for the specified batch header.
    /// If a transmission does not exist, it will be fetched from the specified peer IP.
    async fn fetch_missing_transmissions(
        &self,
        peer_ip: SocketAddr,
        batch_header: &BatchHeader<N>,
    ) -> Result<HashMap<TransmissionID<N>, Transmission<N>>> {
        // If the round is <= the GC round, return early.
        if batch_header.round() <= self.storage.gc_round() {
            return Ok(Default::default());
        }

        // Ensure this batch ID is new.
        if self.storage.contains_batch(batch_header.batch_id()) {
            bail!("Batch for round {} from peer has already been processed", batch_header.round())
        }

        // Retrieve the workers.
        let workers = self.workers.clone();

        // Initialize a list for the transmissions.
        let mut fetch_transmissions = FuturesUnordered::new();

        // Retrieve the number of workers.
        let num_workers = self.num_workers();
        // Iterate through the transmission IDs.
        for transmission_id in batch_header.transmission_ids() {
            // If the transmission does not exist in storage, proceed to fetch the transmission.
            if !self.storage.contains_transmission(*transmission_id) {
                // Determine the worker ID.
                let Ok(worker_id) = assign_to_worker(*transmission_id, num_workers) else {
                    bail!("Unable to assign transmission ID '{transmission_id}' to a worker")
                };
                // Retrieve the worker.
                let Some(worker) = workers.get(worker_id as usize) else { bail!("Unable to find worker {worker_id}") };
                // Push the callback onto the list.
                fetch_transmissions.push(worker.get_or_fetch_transmission(peer_ip, *transmission_id));
            }
        }

        // Initialize a set for the transmissions.
        let mut transmissions = HashMap::with_capacity(fetch_transmissions.len());
        // Wait for all of the transmissions to be fetched.
        while let Some(result) = fetch_transmissions.next().await {
            // Retrieve the transmission.
            let (transmission_id, transmission) = result?;
            // Insert the transmission into the set.
            transmissions.insert(transmission_id, transmission);
        }
        // Return the transmissions.
        Ok(transmissions)
    }

    /// Fetches any missing previous certificates for the specified batch header from the specified peer.
    async fn fetch_missing_previous_certificates(
        &self,
        peer_ip: SocketAddr,
        batch_header: &BatchHeader<N>,
    ) -> Result<HashSet<BatchCertificate<N>>> {
        // Retrieve the round.
        let round = batch_header.round();
        // If the previous round is 0, or is <= the GC round, return early.
        if round == 1 || round <= self.storage.gc_round() + 1 {
            return Ok(Default::default());
        }

        // Initialize a list for the missing previous certificates.
        let mut fetch_certificates = FuturesUnordered::new();
        // Iterate through the previous certificate IDs.
        for certificate_id in batch_header.previous_certificate_ids() {
            // Check if the certificate already exists in the ledger.
            // TODO (howardwu): Add a ledger service.
            // If we do not have the previous certificate, request it.
            if !self.storage.contains_certificate(*certificate_id) {
                trace!("Primary - Found a new certificate ID for round {round} from peer '{peer_ip}'");
                // TODO (howardwu): Limit the number of open requests we send to a peer.
                // Send an certificate request to the peer.
                fetch_certificates.push(self.send_certificate_request(peer_ip, *certificate_id));
            }
        }

        // If there are no missing previous certificates, return early.
        match fetch_certificates.is_empty() {
            true => return Ok(Default::default()),
            false => trace!(
                "Fetching {} missing previous certificates for round {round} from peer '{peer_ip}'...",
                fetch_certificates.len(),
            ),
        }

        // Initialize a set for the missing previous certificates.
        let mut missing_previous_certificates = HashSet::with_capacity(fetch_certificates.len());
        // Wait for all of the missing previous certificates to be fetched.
        while let Some(result) = fetch_certificates.next().await {
            // Insert the missing previous certificate into the set.
            missing_previous_certificates.insert(result?);
        }
        debug!(
            "Fetched {} missing previous certificates for round {round} from peer '{peer_ip}'",
            missing_previous_certificates.len(),
        );
        // Return the missing previous certificates.
        Ok(missing_previous_certificates)
    }

    /// Sends an certificate request to the specified peer.
    async fn send_certificate_request(
        &self,
        peer_ip: SocketAddr,
        certificate_id: Field<N>,
    ) -> Result<BatchCertificate<N>> {
        // Initialize a oneshot channel.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Insert the certificate ID into the pending queue.
        if self.pending.insert(certificate_id, peer_ip, Some(callback_sender)) {
            // TODO (howardwu): Limit the number of open requests we send to a peer.
            // Send the certificate request to the peer.
            self.gateway.send(peer_ip, Event::CertificateRequest(certificate_id.into()));
        }
        // Wait for the certificate to be fetched.
        match timeout(Duration::from_secs(10), callback_receiver).await {
            // If the certificate was fetched, return it.
            Ok(result) => Ok(result?),
            // If the certificate was not fetched, return an error.
            Err(e) => bail!("Unable to fetch batch certificate - (timeout) {e}"),
        }
    }

    /// Handles the incoming certificate response.
    /// This method ensures the certificate response is well-formed and matches the certificate ID.
    fn finish_certificate_request(&self, peer_ip: SocketAddr, response: CertificateResponse<N>) {
        let certificate = response.certificate;
        // Check if the peer IP exists in the pending queue for the given certificate ID.
        let exists = self.pending.get(certificate.certificate_id()).unwrap_or_default().contains(&peer_ip);
        // If the peer IP exists, finish the pending request.
        if exists {
            // TODO: Validate the certificate.
            // Remove the certificate ID from the pending queue.
            self.pending.remove(certificate.certificate_id(), Some(certificate));
        }
    }

    /// Handles the incoming certificate request.
    fn send_certificate_response(&self, peer_ip: SocketAddr, request: CertificateRequest<N>) {
        // Attempt to retrieve the certificate.
        if let Some(certificate) = self.storage.get_certificate(request.certificate_id) {
            // Send the certificate response to the peer.
            self.gateway.send(peer_ip, Event::CertificateResponse(certificate.into()));
        }
    }

    /// Updates the committee to the next round, returning the next round number.
    fn update_committee_to_next_round(&self) {
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
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the primary.
    pub async fn shut_down(&self) {
        trace!("Shutting down the primary...");
        // Iterate through the workers.
        self.workers.iter().for_each(|worker| {
            // Shut down the worker.
            worker.shut_down();
        });
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
        // Close the gateway.
        self.gateway.shut_down().await;
    }
}
