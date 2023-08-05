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
    event::{BatchPropose, BatchSignature, CertificateRequest, CertificateResponse, Event},
    helpers::{
        assign_to_worker,
        assign_to_workers,
        init_worker_channels,
        now,
        BFTSender,
        Pending,
        PrimaryReceiver,
        PrimarySender,
        Proposal,
        Storage,
    },
    Gateway,
    Transport,
    Worker,
    MAX_BATCH_DELAY,
    MAX_TRANSMISSIONS_PER_BATCH,
    MAX_WORKERS,
};
use snarkos_account::Account;
use snarkos_node_narwhal_ledger_service::LedgerService;
use snarkvm::{
    console::prelude::*,
    ledger::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        narwhal::{BatchCertificate, BatchHeader, Data, Transmission, TransmissionID},
    },
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
use tokio::{
    sync::{oneshot, OnceCell},
    task::{self, JoinHandle},
    time::timeout,
};

/// A helper type for an optional proposed batch.
pub type ProposedBatch<N> = RwLock<Option<Proposal<N>>>;
/// A helper type for the ledger service.
pub type Ledger<N> = Arc<dyn LedgerService<N>>;

#[derive(Clone)]
pub struct Primary<N: Network> {
    /// The gateway.
    gateway: Gateway<N>,
    /// The storage.
    storage: Storage<N>,
    /// The ledger service.
    ledger: Ledger<N>,
    /// The workers.
    workers: Arc<[Worker<N>]>,
    /// The BFT sender.
    bft_sender: Arc<OnceCell<BFTSender<N>>>,
    /// The batch proposal, if the primary is currently proposing a batch.
    proposed_batch: Arc<ProposedBatch<N>>,
    /// The pending certificates queue.
    pending: Arc<Pending<Field<N>, BatchCertificate<N>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Primary<N> {
    /// Initializes a new primary instance.
    pub fn new(
        account: Account<N>,
        storage: Storage<N>,
        ledger: Ledger<N>,
        ip: Option<SocketAddr>,
        dev: Option<u16>,
    ) -> Result<Self> {
        Ok(Self {
            gateway: Gateway::new(account, storage.clone(), ip, dev)?,
            storage,
            ledger,
            workers: Arc::from(vec![]),
            bft_sender: Default::default(),
            proposed_batch: Default::default(),
            pending: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the primary instance.
    pub async fn run(
        &mut self,
        primary_sender: PrimarySender<N>,
        primary_receiver: PrimaryReceiver<N>,
        bft_sender: Option<BFTSender<N>>,
    ) -> Result<()> {
        info!("Starting the primary instance of the memory pool...");

        // Set the primary sender.
        self.gateway.set_primary_sender(primary_sender);
        // Set the BFT sender.
        if let Some(bft_sender) = bft_sender {
            self.bft_sender.set(bft_sender).expect("BFT sender already set");
        }

        // Construct a map of the worker senders.
        let mut tx_workers = IndexMap::new();
        // Construct a map for the workers.
        let mut workers = Vec::new();
        // Initialize the workers.
        for id in 0..MAX_WORKERS {
            // Construct the worker channels.
            let (tx_worker, rx_worker) = init_worker_channels();
            // Construct the worker instance.
            let worker = Worker::new(
                id,
                Arc::new(self.gateway.clone()),
                self.storage.clone(),
                self.ledger.clone(),
                self.proposed_batch.clone(),
            )?;
            // Run the worker instance.
            worker.run(rx_worker);
            // Add the worker to the list of workers.
            workers.push(worker);
            // Add the worker sender to the map.
            tx_workers.insert(id, tx_worker);
        }
        // Set the workers.
        self.workers = Arc::from(workers);

        // Initialize the gateway.
        self.gateway.run(tx_workers).await;

        // Start the primary handlers.
        self.start_handlers(primary_receiver);

        Ok(())
    }

    /// Returns the current round.
    pub fn current_round(&self) -> u64 {
        self.storage.current_round()
    }

    /// Returns the gateway.
    pub const fn gateway(&self) -> &Gateway<N> {
        &self.gateway
    }

    /// Returns the storage.
    pub const fn storage(&self) -> &Storage<N> {
        &self.storage
    }

    /// Returns the ledger.
    pub const fn ledger(&self) -> &Ledger<N> {
        &self.ledger
    }

    /// Returns the number of workers.
    pub fn num_workers(&self) -> u8 {
        u8::try_from(self.workers.len()).expect("Too many workers")
    }

    /// Returns the workers.
    pub const fn workers(&self) -> &Arc<[Worker<N>]> {
        &self.workers
    }

    /// Returns the batch proposal of our primary, if one currently exists.
    pub fn proposed_batch(&self) -> &Arc<ProposedBatch<N>> {
        &self.proposed_batch
    }
}

impl<N: Network> Primary<N> {
    /// Returns the number of unconfirmed transmissions.
    pub fn num_unconfirmed_transmissions(&self) -> usize {
        self.workers.iter().map(|worker| worker.num_transmissions()).sum()
    }

    /// Returns the number of unconfirmed ratifications.
    pub fn num_unconfirmed_ratifications(&self) -> usize {
        self.workers.iter().map(|worker| worker.num_ratifications()).sum()
    }

    /// Returns the number of solutions.
    pub fn num_unconfirmed_solutions(&self) -> usize {
        self.workers.iter().map(|worker| worker.num_solutions()).sum()
    }

    /// Returns the number of unconfirmed transactions.
    pub fn num_unconfirmed_transactions(&self) -> usize {
        self.workers.iter().map(|worker| worker.num_transactions()).sum()
    }
}

impl<N: Network> Primary<N> {
    /// Returns the unconfirmed transmission IDs.
    pub fn unconfirmed_transmission_ids(&self) -> impl '_ + Iterator<Item = TransmissionID<N>> {
        self.workers.iter().flat_map(|worker| worker.transmission_ids())
    }

    /// Returns the unconfirmed transmissions.
    pub fn unconfirmed_transmissions(&self) -> impl '_ + Iterator<Item = (TransmissionID<N>, Transmission<N>)> {
        self.workers.iter().flat_map(|worker| worker.transmissions())
    }

    /// Returns the unconfirmed solutions.
    pub fn unconfirmed_solutions(&self) -> impl '_ + Iterator<Item = (PuzzleCommitment<N>, Data<ProverSolution<N>>)> {
        self.workers.iter().flat_map(|worker| worker.solutions())
    }

    /// Returns the unconfirmed transactions.
    pub fn unconfirmed_transactions(&self) -> impl '_ + Iterator<Item = (N::TransactionID, Data<Transaction<N>>)> {
        self.workers.iter().flat_map(|worker| worker.transactions())
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
    pub async fn propose_batch(&self) -> Result<()> {
        // Check if the proposed batch has expired, and clear it if it has expired.
        self.check_proposed_batch_for_expiration().await?;
        // If there is a batch being proposed already,
        // rebroadcast the batch header to the non-signers, and return early.
        if let Some(proposal) = self.proposed_batch.read().as_ref() {
            // Construct the event.
            // TODO(ljedrz): the BatchHeader should be serialized only once in advance before being sent to non-signers.
            let event = Event::BatchPropose(proposal.batch_header().clone().into());
            // Iterate through the non-signers.
            for address in proposal.nonsigners() {
                // Resolve the address to the peer IP.
                match self.gateway.resolver().get_peer_ip_for_address(address) {
                    // Broadcast the batch to all validators for signing.
                    Some(peer_ip) => {
                        debug!("Resending batch proposal for round {} to peer '{peer_ip}'", proposal.round());
                        // Broadcast the event.
                        self.gateway.send(peer_ip, event.clone());
                    }
                    None => continue,
                }
            }
            // Return early.
            return Ok(());
        }

        // Retrieve the current round.
        let round = self.current_round();
        // Compute the previous round.
        let previous_round = round.saturating_sub(1);
        // Ensure the primary has not proposed a batch for this round before.
        if self.storage.contains_certificate_in_round_from(round, self.gateway.account().address()) {
            // If a BFT sender was provided, attempt to advance the current round.
            if let Some(bft_sender) = self.bft_sender.get() {
                if let Err(e) = self.send_primary_round_to_bft(bft_sender).await {
                    warn!("Failed to update the BFT to the next round: {e}");
                    return Err(e);
                }
            }
            bail!("Primary is safely skipping (round {round} was already certified)")
        }
        // Retrieve the previous certificates.
        let previous_certificates = self.storage.get_certificates_for_round(previous_round);

        // Check if the batch is ready to be proposed.
        // Note: The primary starts at round 1, and round 0 contains no certificates, by definition.
        let mut is_ready = previous_round == 0;
        // If the previous round is not 0, check if the previous certificates have reached the quorum threshold.
        if previous_round > 0 {
            // Retrieve the committee for the round.
            let Ok(committee) = self.ledger.get_committee_for_round(previous_round) else {
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

        // Determined the required number of transmissions per worker.
        let num_transmissions_per_worker = MAX_TRANSMISSIONS_PER_BATCH / self.num_workers() as usize;
        // Take the transmissions from the workers.
        let mut transmissions: IndexMap<_, _> = Default::default();
        for worker in self.workers.iter() {
            transmissions.extend(worker.take_candidates(num_transmissions_per_worker).await);
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
        // Generate the local timestamp for batch
        let timestamp = now();
        // Prepare the transmission IDs.
        let transmission_ids = transmissions.keys().copied().collect();
        // Prepare the certificate IDs.
        let certificate_ids = previous_certificates.into_iter().map(|c| c.certificate_id()).collect();
        // Sign the batch header.
        let batch_header = BatchHeader::new(private_key, round, timestamp, transmission_ids, certificate_ids, rng)?;
        // Construct the proposal.
        let proposal = Proposal::new(self.ledger.current_committee()?, batch_header.clone(), transmissions)?;
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

        // TODO (howardwu): Ensure I have not signed this round for this author before. If so, do not sign.

        // Deserialize the batch header.
        let batch_header = task::spawn_blocking(move || batch_header.deserialize_blocking()).await??;
        // Ensure the round matches in the batch header.
        if batch_round != batch_header.round() {
            bail!("Malicious peer - proposed round {batch_round}, but sent batch for round {}", batch_header.round());
        }

        // If the peer is ahead, use the batch header to sync up to the peer.
        let transmissions = self.sync_with_header_from_peer(peer_ip, &batch_header).await?;

        // Ensure the batch is for the current round.
        // This method must be called after fetching previous certificates (above),
        // and prior to checking the batch header (below).
        self.ensure_is_signing_round(batch_round)?;

        // Ensure the batch header from the peer is valid.
        let missing_transmissions = self.storage.check_batch_header(&batch_header, transmissions)?;
        // Inserts the missing transmissions into the workers.
        self.insert_missing_transmissions_into_workers(peer_ip, missing_transmissions.into_iter())?;

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
        self.check_proposed_batch_for_expiration().await?;

        // Retrieve the signature and timestamp.
        let BatchSignature { batch_id, signature, timestamp } = batch_signature;

        let proposal = {
            // Acquire the write lock.
            let mut proposed_batch = self.proposed_batch.write();
            // Add the signature to the batch, and determine if the batch is ready to be certified.
            match proposed_batch.as_mut() {
                Some(proposal) => {
                    // Ensure the batch ID matches the currently proposed batch ID.
                    if proposal.batch_id() != batch_id {
                        match self.storage.contains_batch(batch_id) {
                            true => bail!("This batch was already certified"),
                            false => bail!("Unknown batch ID '{batch_id}'"),
                        }
                    }
                    // Retrieve the address of the peer.
                    match self.gateway.resolver().get_address(peer_ip) {
                        // Add the signature to the batch.
                        Some(signer) => proposal.add_signature(signer, signature, timestamp)?,
                        None => bail!("Signature is from a disconnected peer"),
                    };
                    info!("Added a batch signature from peer '{peer_ip}'");
                    // Check if the batch is ready to be certified.
                    if !proposal.is_quorum_threshold_reached() {
                        // If the batch is not ready to be certified, return early.
                        return Ok(());
                    }
                }
                // There is no proposed batch, so return early.
                None => return Ok(()),
            };
            // Retrieve the batch proposal, clearing the proposed batch.
            match proposed_batch.take() {
                Some(proposal) => proposal,
                None => return Ok(()),
            }
        };

        /* Proceeding to certify the batch. */

        info!("Quorum threshold reached - Preparing to certify our batch...");

        // Store the certified batch and broadcast it to all validators.
        // If there was an error storing the certificate, reinsert the transmissions back into the ready queue.
        if let Err(e) = self.store_and_broadcast_certificate(&proposal).await {
            // Reinsert the transmissions back into the ready queue for the next proposal.
            self.reinsert_transmissions_into_workers(proposal)?;
            return Err(e);
        }
        Ok(())
    }

    /// Processes a batch certificate from a peer.
    ///
    /// This method performs the following steps:
    /// 1. Stores the given batch certificate, after ensuring it is valid.
    /// 2. If there are enough certificates to reach quorum threshold for the current round,
    ///  then proceed to advance to the next round.
    async fn process_batch_certificate_from_peer(
        &self,
        peer_ip: SocketAddr,
        certificate: BatchCertificate<N>,
    ) -> Result<()> {
        // Store the certificate, after ensuring it is valid.
        self.sync_with_certificate_from_peer(peer_ip, certificate).await?;
        // If there are enough certificates to reach quorum threshold for the current round,
        // then proceed to advance to the next round.
        self.try_advance_to_next_round().await
    }
}

impl<N: Network> Primary<N> {
    /// Starts the primary handlers.
    fn start_handlers(&self, primary_receiver: PrimaryReceiver<N>) {
        let PrimaryReceiver {
            mut rx_batch_propose,
            mut rx_batch_signature,
            mut rx_batch_certified,
            mut rx_certificate_request,
            mut rx_certificate_response,
            mut rx_unconfirmed_solution,
            mut rx_unconfirmed_transaction,
        } = primary_receiver;

        // Start the batch proposer.
        let self_ = self.clone();
        self.spawn(async move {
            loop {
                // Sleep briefly, but longer than if there were no batch.
                tokio::time::sleep(Duration::from_millis(MAX_BATCH_DELAY)).await;
                // If there is no proposed batch, attempt to propose a batch.
                if let Err(e) = self_.propose_batch().await {
                    warn!("Cannot propose a batch - {e}");
                }
            }
        });

        // Process the proposed batch.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_propose)) = rx_batch_propose.recv().await {
                if let Err(e) = self_.process_batch_propose_from_peer(peer_ip, batch_propose).await {
                    warn!("Cannot sign a batch from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the batch signature.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_signature)) = rx_batch_signature.recv().await {
                if let Err(e) = self_.process_batch_signature_from_peer(peer_ip, batch_signature).await {
                    warn!("Cannot store a signature from peer '{peer_ip}' - {e}");
                }
            }
        });

        // Process the certified batch.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, batch_certificate)) = rx_batch_certified.recv().await {
                // Deserialize the batch certificate.
                let Ok(Ok(batch_certificate)) =
                    task::spawn_blocking(move || batch_certificate.deserialize_blocking()).await
                else {
                    warn!("Failed to deserialize the batch certificate from peer '{peer_ip}'");
                    continue;
                };
                if let Err(e) = self_.process_batch_certificate_from_peer(peer_ip, batch_certificate).await {
                    warn!("Cannot store a certificate from peer '{peer_ip}' - {e}");
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
            while let Some((puzzle_commitment, prover_solution, callback)) = rx_unconfirmed_solution.recv().await {
                // Compute the worker ID.
                let Ok(worker_id) = assign_to_worker(puzzle_commitment, self_.num_workers()) else {
                    error!("Unable to determine the worker ID for the unconfirmed solution");
                    continue;
                };
                // Retrieve the worker.
                let worker = &self_.workers[worker_id as usize];
                // Process the unconfirmed solution.
                let result = worker.process_unconfirmed_solution(puzzle_commitment, prover_solution).await;
                // Send the result to the callback.
                callback.send(result).ok();
            }
        });

        // Process the unconfirmed transactions.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((transaction_id, transaction, callback)) = rx_unconfirmed_transaction.recv().await {
                // Compute the worker ID.
                let Ok(worker_id) = assign_to_worker::<N>(&transaction_id, self_.num_workers()) else {
                    error!("Unable to determine the worker ID for the unconfirmed transaction");
                    continue;
                };
                // Retrieve the worker.
                let worker = &self_.workers[worker_id as usize];
                // Process the unconfirmed transaction.
                let result = worker.process_unconfirmed_transaction(transaction_id, transaction).await;
                // Send the result to the callback.
                callback.send(result).ok();
            }
        });
    }

    /// Checks if the proposed batch is expired, and clears the proposed batch if it has expired.
    async fn check_proposed_batch_for_expiration(&self) -> Result<()> {
        // Check if the proposed batch is expired.
        let is_expired = self.proposed_batch.read().as_ref().map_or(false, Proposal::is_expired);
        // If the batch is expired, clear it.
        if is_expired {
            // Reset the proposed batch.
            let proposal = self.proposed_batch.write().take();
            if let Some(proposal) = proposal {
                self.reinsert_transmissions_into_workers(proposal)?;
            }
            // If there are enough certificates to reach quorum threshold for the current round,
            // then proceed to advance to the next round.
            self.try_advance_to_next_round().await?;
        }
        Ok(())
    }

    /// If the current round has reached quorum threshold, then advance to the next round.
    async fn try_advance_to_next_round(&self) -> Result<()> {
        // Retrieve the current committee.
        let current_committee = self.ledger.current_committee()?;
        // Retrieve the current round.
        let current_round = self.current_round();
        // Retrieve the certificates.
        let certificates = self.storage.get_certificates_for_round(current_round);
        // Construct a set over the authors.
        let authors = certificates.iter().map(BatchCertificate::author).collect();
        // Check if the certificates have reached the quorum threshold.
        let is_quorum = current_committee.is_quorum_threshold_reached(&authors);

        // Determine if we are currently proposing a round.
        // Note: This is important, because while our peers have advanced,
        // they may not be proposing yet, and thus still able to sign our proposed batch.
        let is_proposing = self.proposed_batch.read().is_some();

        // Determine whether to advance to the next round.
        if is_quorum && !is_proposing {
            // Update to the next committee in storage.
            // self.storage.increment_committee_to_next_round()?;
            // // If we have reached the quorum threshold, then proceed to the next round.
            // self.try_advance_to_next_round_in_narwhal().await?;
            // If we have reached the quorum threshold, then proceed to the next round.
            self.update_committee_to_round(current_round + 1).await?;
            // // Start proposing a batch for the next round.
            // self.propose_batch().await?;
        }
        Ok(())
    }

    /// Updates the committee to the specified round.
    ///
    /// This method should only be called after processing a proposed batch, or in catching up,
    /// and thus enforces that the proposed batch is cleared.
    async fn update_committee_to_round(&self, next_round: u64) -> Result<()> {
        // Iterate until the penultimate round is reached.
        while self.current_round() < next_round.saturating_sub(1) {
            // Update to the next committee in storage.
            // TODO (howardwu): Fix to increment to the next round.
            self.storage.increment_to_next_round()?;
            // Clear the proposed batch.
            *self.proposed_batch.write() = None;
        }
        // Attempt to advance to the next round.
        if self.current_round() < next_round {
            // If a BFT sender was provided, send the current round to the BFT.
            if let Some(bft_sender) = self.bft_sender.get() {
                if let Err(e) = self.send_primary_round_to_bft(bft_sender).await {
                    warn!("Failed to update the BFT to the next round: {e}");
                    return Err(e);
                }
            }
            // Otherwise, handle the Narwhal case.
            else {
                // Update to the next committee in storage.
                // TODO (howardwu): Fix to increment to the next round.
                self.storage.increment_to_next_round()?;
            }
            // Clear the proposed batch.
            *self.proposed_batch.write() = None;
        }
        Ok(())
    }

    /// Sends the current round to the BFT.
    async fn send_primary_round_to_bft(&self, bft_sender: &BFTSender<N>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the current round to the BFT.
        bft_sender.tx_primary_round.send((self.current_round(), callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }

    /// Sends the batch certificate to the BFT.
    async fn send_primary_certificate_to_bft(
        &self,
        bft_sender: &BFTSender<N>,
        certificate: BatchCertificate<N>,
    ) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the certificate to the BFT.
        bft_sender.tx_primary_certificate.send((certificate, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }

    /// Ensures the primary is signing for the specified batch round.
    /// This method is used to ensure: for a given round, as soon as the primary starts proposing,
    /// it will no longer sign for the previous round (as it has enough previous certificates to proceed).
    fn ensure_is_signing_round(&self, batch_round: u64) -> Result<()> {
        // Retrieve the current round.
        let current_round = self.current_round();
        // Ensure the batch round is within GC range of the current round.
        if current_round + self.storage.max_gc_rounds() <= batch_round {
            bail!("Round {batch_round} is too far in the future")
        }
        // Ensure the batch round is at or one before the current round.
        // Intuition: Our primary has moved on to the next round, but has not necessarily started proposing,
        // so we can still sign for the previous round. If we have started proposing, the next check will fail.
        if current_round > batch_round + 1 {
            bail!("Primary is on round {current_round}, and no longer signing for round {batch_round}")
        }
        // Check if the primary is still signing for the batch round.
        if let Some(signing_round) = self.proposed_batch.read().as_ref().map(|proposal| proposal.round()) {
            if signing_round > batch_round {
                bail!("Our primary at round {signing_round} is no longer signing for round {batch_round}")
            }
        }
        Ok(())
    }

    /// Stores the certified batch and broadcasts it to all validators, returning the certificate.
    async fn store_and_broadcast_certificate(&self, proposal: &Proposal<N>) -> Result<()> {
        // Create the batch certificate and transmissions.
        let (certificate, transmissions) = proposal.to_certificate()?;
        // Convert the transmissions into a HashMap.
        // Note: Do not change the `Proposal` to use a HashMap. The ordering there is necessary for safety.
        let transmissions = transmissions.into_iter().collect::<HashMap<_, _>>();
        // Store the certified batch.
        self.storage.insert_certificate(certificate.clone(), transmissions)?;
        debug!("Stored certificate for round {}", certificate.round());
        // Broadcast the certified batch to all validators.
        self.gateway.broadcast(Event::BatchCertified(certificate.clone().into()));
        // If a BFT sender was provided, send the certificate to the BFT.
        if let Some(bft_sender) = self.bft_sender.get() {
            // Await the callback to continue.
            if let Err(e) = self.send_primary_certificate_to_bft(bft_sender, certificate.clone()).await {
                warn!("Failed to update the BFT DAG from primary: {e}");
                return Err(e);
            };
        }
        // Log the certified batch.
        let num_transmissions = certificate.transmission_ids().len();
        let round = certificate.round();
        info!("\n\nOur batch with {num_transmissions} transmissions for round {round} was certified!\n");
        // Update the committee to the next round.
        // self.update_committee_to_next_round().await
        self.update_committee_to_round(round + 1).await
    }

    /// Inserts the missing transmissions from the proposal into the workers.
    fn insert_missing_transmissions_into_workers(
        &self,
        peer_ip: SocketAddr,
        transmissions: impl Iterator<Item = (TransmissionID<N>, Transmission<N>)>,
    ) -> Result<()> {
        // Insert the transmissions into the workers.
        assign_to_workers(&self.workers, transmissions, |worker, transmission_id, transmission| {
            worker.process_transmission_from_peer(peer_ip, transmission_id, transmission);
        })
    }

    /// Re-inserts the transmissions from the proposal into the workers.
    fn reinsert_transmissions_into_workers(&self, proposal: Proposal<N>) -> Result<()> {
        // Re-insert the transmissions into the workers.
        assign_to_workers(
            &self.workers,
            proposal.into_transmissions().into_iter(),
            |worker, transmission_id, transmission| {
                worker.reinsert(transmission_id, transmission);
            },
        )
    }

    /// Recursively stores a given batch certificate, after ensuring:
    ///   - Ensure the round matches the committee round.
    ///   - Ensure the address is a member of the committee.
    ///   - Ensure the timestamp is within range.
    ///   - Ensure we have all of the transmissions.
    ///   - Ensure we have all of the previous certificates.
    ///   - Ensure the previous certificates are for the previous round (i.e. round - 1).
    ///   - Ensure the previous certificates have reached the quorum threshold.
    ///   - Ensure we have not already signed the batch ID.
    #[async_recursion]
    async fn sync_with_certificate_from_peer(
        &self,
        peer_ip: SocketAddr,
        certificate: BatchCertificate<N>,
    ) -> Result<()> {
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

        // If the peer is ahead, use the batch header to sync up to the peer.
        let missing_transmissions = self.sync_with_header_from_peer(peer_ip, batch_header).await?;

        // Check if the certificate needs to be stored.
        if !self.storage.contains_certificate(certificate.certificate_id()) {
            // Store the batch certificate.
            self.storage.insert_certificate(certificate.clone(), missing_transmissions)?;
            debug!("Stored certificate for round {batch_round} from peer '{peer_ip}'");
            // If a BFT sender was provided, send the certificate to the BFT.
            if let Some(bft_sender) = self.bft_sender.get() {
                // Await the callback to continue.
                if let Err(e) = self.send_primary_certificate_to_bft(bft_sender, certificate.clone()).await {
                    warn!("Failed to update the BFT DAG from sync: {e}");
                    return Err(e);
                };
            }
        }
        Ok(())
    }

    // TODO (howardwu): This method is a mess. There are many redundant checks and logic. Ignore these until design is stable.
    /// Recursively syncs using the given batch header.
    async fn sync_with_header_from_peer(
        &self,
        peer_ip: SocketAddr,
        batch_header: &BatchHeader<N>,
    ) -> Result<HashMap<TransmissionID<N>, Transmission<N>>> {
        // Retrieve the batch round.
        let batch_round = batch_header.round();

        // If the certificate round is outdated, do not store it.
        if batch_round <= self.storage.gc_round() {
            bail!("Round {batch_round} is too far in the past")
        }

        // Check if our primary should move to the next round.
        let is_behind_schedule = batch_round > self.current_round(); // TODO: Check if threshold is reached.
        // Check if our primary is far behind the peer.
        let is_peer_far_in_future = batch_round > self.current_round() + self.storage.max_gc_rounds();
        // If our primary is far behind the peer, update our committee to the batch round.
        if is_behind_schedule || is_peer_far_in_future {
            // TODO (howardwu): Guard this to increment after quorum threshold is reached.
            // TODO (howardwu): After bullshark is implemented, we must use Aleo blocks to guide us to `tip-50` to know the committee.
            // If the batch round is greater than the current committee round, update the committee.
            // self.update_committee_to_round_catch_up(batch_round).await?;
            self.update_committee_to_round(batch_round).await?;
        }

        // // Ensure this batch does not contain already committed transmissions from past rounds.
        // if batch_header.transmission_ids().iter().any(|id| self.storage.contains_transmission(*id)) {
        //     bail!("Batch contains already transmissions from past rounds");
        // }
        // Ensure this batch does not contain already committed transmissions in the ledger.
        // TODO: Add a ledger service.

        // Ensure the primary has all of the previous certificates.
        let missing_certificates = self.fetch_missing_previous_certificates(peer_ip, batch_header).await?;
        // Ensure the primary has all of the transmissions.
        let missing_transmissions = self.fetch_missing_transmissions(peer_ip, batch_header).await?;

        // Iterate through the missing certificates.
        for batch_certificate in missing_certificates {
            // Store the batch certificate (recursively fetching any missing previous certificates).
            self.sync_with_certificate_from_peer(peer_ip, batch_certificate).await?;
        }
        Ok(missing_transmissions)
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
            if self.ledger.contains_certificate(certificate_id)? {
                continue;
            }
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
            // Send the certificate request to the peer.
            self.gateway.send(peer_ip, Event::CertificateRequest(certificate_id.into()));
        }
        // Wait for the certificate to be fetched.
        match timeout(Duration::from_millis(MAX_BATCH_DELAY), callback_receiver).await {
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

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the primary.
    pub async fn shut_down(&self) {
        trace!("Shutting down the primary...");
        // Shut down the workers.
        self.workers.iter().for_each(|worker| worker.shut_down());
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
        // Close the gateway.
        self.gateway.shut_down().await;
    }
}
