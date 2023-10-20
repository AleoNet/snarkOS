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
        fmt_id,
        init_bft_channels,
        now,
        BFTReceiver,
        ConsensusSender,
        PrimaryReceiver,
        PrimarySender,
        Storage,
        DAG,
    },
    Primary,
    MAX_LEADER_CERTIFICATE_DELAY,
};
use snarkos_account::Account;
use snarkos_node_bft_ledger_service::LedgerService;
use snarkvm::{
    console::account::Address,
    ledger::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        committee::Committee,
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
    },
    prelude::{bail, ensure, Field, Network, Result},
};

use colored::Colorize;
use indexmap::{IndexMap, IndexSet};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    net::SocketAddr,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};
use tokio::{
    sync::{oneshot, Mutex as TMutex, OnceCell},
    task::JoinHandle,
};

#[derive(Clone)]
pub struct BFT<N: Network> {
    /// The primary.
    primary: Primary<N>,
    /// The DAG.
    dag: Arc<RwLock<DAG<N>>>,
    /// The batch certificate of the leader from the current even round, if one was present.
    leader_certificate: Arc<RwLock<Option<BatchCertificate<N>>>>,
    /// The timer for the leader certificate to be received.
    leader_certificate_timer: Arc<AtomicI64>,
    /// The consensus sender.
    consensus_sender: Arc<OnceCell<ConsensusSender<N>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// The BFT lock.
    lock: Arc<TMutex<()>>,
}

impl<N: Network> BFT<N> {
    /// Initializes a new instance of the BFT.
    pub fn new(
        account: Account<N>,
        storage: Storage<N>,
        ledger: Arc<dyn LedgerService<N>>,
        ip: Option<SocketAddr>,
        trusted_validators: &[SocketAddr],
        dev: Option<u16>,
    ) -> Result<Self> {
        Ok(Self {
            primary: Primary::new(account, storage, ledger, ip, trusted_validators, dev)?,
            dag: Default::default(),
            leader_certificate: Default::default(),
            leader_certificate_timer: Default::default(),
            consensus_sender: Default::default(),
            handles: Default::default(),
            lock: Default::default(),
        })
    }

    /// Run the BFT instance.
    pub async fn run(
        &mut self,
        consensus_sender: Option<ConsensusSender<N>>,
        primary_sender: PrimarySender<N>,
        primary_receiver: PrimaryReceiver<N>,
    ) -> Result<()> {
        info!("Starting the BFT instance...");
        // Initialize the BFT channels.
        let (bft_sender, bft_receiver) = init_bft_channels::<N>();
        // First, start the BFT handlers.
        self.start_handlers(bft_receiver);
        // Next, run the primary instance.
        self.primary.run(Some(bft_sender), primary_sender, primary_receiver).await?;
        // Lastly, set the consensus sender.
        // Note: This ensures during initial syncing, that the BFT does not advance the ledger.
        if let Some(consensus_sender) = consensus_sender {
            self.consensus_sender.set(consensus_sender).expect("Consensus sender already set");
        }
        Ok(())
    }

    /// Returns the primary.
    pub const fn primary(&self) -> &Primary<N> {
        &self.primary
    }

    /// Returns the storage.
    pub const fn storage(&self) -> &Storage<N> {
        self.primary.storage()
    }

    /// Returns the ledger.
    pub fn ledger(&self) -> &Arc<dyn LedgerService<N>> {
        self.primary.ledger()
    }

    /// Returns the leader of the current even round, if one was present.
    pub fn leader(&self) -> Option<Address<N>> {
        self.leader_certificate.read().as_ref().map(|certificate| certificate.author())
    }

    /// Returns the certificate of the leader from the current even round, if one was present.
    pub const fn leader_certificate(&self) -> &Arc<RwLock<Option<BatchCertificate<N>>>> {
        &self.leader_certificate
    }
}

impl<N: Network> BFT<N> {
    /// Returns the number of unconfirmed transmissions.
    pub fn num_unconfirmed_transmissions(&self) -> usize {
        self.primary.num_unconfirmed_transmissions()
    }

    /// Returns the number of unconfirmed ratifications.
    pub fn num_unconfirmed_ratifications(&self) -> usize {
        self.primary.num_unconfirmed_ratifications()
    }

    /// Returns the number of solutions.
    pub fn num_unconfirmed_solutions(&self) -> usize {
        self.primary.num_unconfirmed_solutions()
    }

    /// Returns the number of unconfirmed transactions.
    pub fn num_unconfirmed_transactions(&self) -> usize {
        self.primary.num_unconfirmed_transactions()
    }
}

impl<N: Network> BFT<N> {
    /// Returns the unconfirmed transmission IDs.
    pub fn unconfirmed_transmission_ids(&self) -> impl '_ + Iterator<Item = TransmissionID<N>> {
        self.primary.unconfirmed_transmission_ids()
    }

    /// Returns the unconfirmed transmissions.
    pub fn unconfirmed_transmissions(&self) -> impl '_ + Iterator<Item = (TransmissionID<N>, Transmission<N>)> {
        self.primary.unconfirmed_transmissions()
    }

    /// Returns the unconfirmed solutions.
    pub fn unconfirmed_solutions(&self) -> impl '_ + Iterator<Item = (PuzzleCommitment<N>, Data<ProverSolution<N>>)> {
        self.primary.unconfirmed_solutions()
    }

    /// Returns the unconfirmed transactions.
    pub fn unconfirmed_transactions(&self) -> impl '_ + Iterator<Item = (N::TransactionID, Data<Transaction<N>>)> {
        self.primary.unconfirmed_transactions()
    }
}

impl<N: Network> BFT<N> {
    /// Stores the certificate in the DAG, and attempts to commit one or more anchors.
    fn update_to_next_round(&self, current_round: u64) -> bool {
        // Ensure the current round is at least the storage round (this is a sanity check).
        let storage_round = self.storage().current_round();
        if current_round < storage_round {
            warn!("BFT is safely skipping an update for round {current_round}, as storage is at round {storage_round}");
            return false;
        }

        // Determine if the BFT is ready to update to the next round.
        let is_ready = match current_round % 2 == 0 {
            true => self.update_leader_certificate_to_even_round(current_round),
            false => self.is_leader_quorum_or_nonleaders_available(current_round),
        };

        // Log whether the round is going to update.
        if current_round % 2 == 0 {
            // Determine if there is a leader certificate.
            if let Some(leader_certificate) = self.leader_certificate.read().as_ref() {
                // Ensure the state of the leader certificate is consistent with the BFT being ready.
                if !is_ready {
                    error!(is_ready, "BFT - A leader certificate was found, but 'is_ready' is false");
                }
                // Log the leader election.
                let leader_round = leader_certificate.round();
                match leader_round == current_round {
                    true => info!("\n\nRound {current_round} elected a leader - {}\n", leader_certificate.author()),
                    false => warn!("BFT failed to elect a leader for round {current_round} (!= {leader_round})"),
                }
            } else {
                match is_ready {
                    true => info!("\n\nRound {current_round} reached quorum without a leader\n"),
                    false => info!("{}", format!("\n\nRound {current_round} did not elect a leader\n").dimmed()),
                }
            }
        }

        // If the BFT is ready, then update to the next round.
        if is_ready {
            // Update to the next round in storage.
            if let Err(e) = self.storage().increment_to_next_round(current_round) {
                warn!("BFT failed to increment to the next round from round {current_round} - {e}");
            }
            // Update the timer for the leader certificate.
            self.leader_certificate_timer.store(now(), Ordering::SeqCst);
        }

        is_ready
    }

    /// Updates the leader certificate to the current even round,
    /// returning `true` if the BFT is ready to update to the next round.
    ///
    /// This method runs on every even round, by determining the leader of the current even round,
    /// and setting the leader certificate to their certificate in the round, if they were present.
    fn update_leader_certificate_to_even_round(&self, even_round: u64) -> bool {
        // Retrieve the current round.
        let current_round = self.storage().current_round();
        // Ensure the current round matches the given round.
        if current_round != even_round {
            warn!("BFT storage (at round {current_round}) is out of sync with the current even round {even_round}");
            return false;
        }

        // If the current round is odd, return false.
        if current_round % 2 != 0 || current_round < 2 {
            error!("BFT cannot update the leader certificate in an odd round");
            return false;
        }

        // Retrieve the certificates for the current round.
        let current_certificates = self.storage().get_certificates_for_round(current_round);
        // If there are no current certificates, set the leader certificate to 'None', and return early.
        if current_certificates.is_empty() {
            // Set the leader certificate to 'None'.
            *self.leader_certificate.write() = None;
            return false;
        }

        // Retrieve the previous committee of the current round.
        let previous_committee = match self.ledger().get_previous_committee_for_round(current_round) {
            Ok(committee) => committee,
            Err(e) => {
                error!("BFT failed to retrieve the previous committee for the even round {current_round} - {e}");
                return false;
            }
        };
        // Determine the leader of the current round.
        let leader = match previous_committee.get_leader(current_round) {
            Ok(leader) => leader,
            Err(e) => {
                error!("BFT failed to compute the leader for the even round {current_round} - {e}");
                return false;
            }
        };
        // Find and set the leader certificate, if the leader was present in the current even round.
        let leader_certificate = current_certificates.iter().find(|certificate| certificate.author() == leader);
        *self.leader_certificate.write() = leader_certificate.cloned();

        self.is_even_round_ready_for_next_round(current_certificates, previous_committee, current_round)
    }

    /// Returns 'true' under one of the following conditions:
    ///  - If the leader certificate is set for the current even round,
    ///  - The timer for the leader certificate has expired, and we can
    ///    achieve quorum threshold (2f + 1) without the leader.
    fn is_even_round_ready_for_next_round(
        &self,
        certificates: IndexSet<BatchCertificate<N>>,
        committee: Committee<N>,
        current_round: u64,
    ) -> bool {
        // If the leader certificate is set for the current even round, return 'true'.
        if let Some(leader_certificate) = self.leader_certificate.read().as_ref() {
            if leader_certificate.round() == current_round {
                return true;
            }
        }
        // If the timer has expired, and we can achieve quorum threshold (2f + 1) without the leader, return 'true'.
        if self.is_timer_expired() {
            debug!("BFT (timer expired) - Checking for quorum threshold (without the leader)");
            // Retrieve the certificate authors.
            let authors = certificates.into_iter().map(|c| c.author()).collect();
            // Determine if the quorum threshold is reached.
            return committee.is_quorum_threshold_reached(&authors);
        }
        // Otherwise, return 'false'.
        false
    }

    /// Returns `true` if the timer for the leader certificate has expired.
    fn is_timer_expired(&self) -> bool {
        self.leader_certificate_timer.load(Ordering::SeqCst) + MAX_LEADER_CERTIFICATE_DELAY <= now()
    }

    /// Returns 'true' if any of the following conditions hold:
    ///  - The leader certificate is 'None'.
    ///  - The leader certificate reached quorum threshold `(2f + 1)` (in the previous certificates in the current round).
    ///  - The leader certificate is not included up to availability threshold `(f + 1)` (in the previous certificates of the current round).
    ///  - The leader certificate timer has expired.
    fn is_leader_quorum_or_nonleaders_available(&self, odd_round: u64) -> bool {
        // Retrieve the current round.
        let current_round = self.storage().current_round();
        // Ensure the current round matches the given round.
        if current_round != odd_round {
            warn!("BFT storage (at round {current_round}) is out of sync with the current odd round {odd_round}");
            return false;
        }
        // If the current round is even, return false.
        if current_round % 2 != 1 {
            error!("BFT does not compute stakes for the leader certificate in an even round");
            return false;
        }

        // Retrieve the leader certificate.
        let Some(leader_certificate) = self.leader_certificate.read().clone() else {
            // If there is no leader certificate for the previous round, return 'true'.
            return true;
        };
        // Retrieve the leader certificate ID.
        let leader_certificate_id = leader_certificate.certificate_id();
        // Retrieve the certificates for the current round.
        let current_certificates = self.storage().get_certificates_for_round(current_round);
        // Retrieve the previous committee of the current round.
        let previous_committee = match self.ledger().get_previous_committee_for_round(current_round) {
            Ok(committee) => committee,
            Err(e) => {
                error!("BFT failed to retrieve the previous committee for the odd round {current_round} - {e}");
                return false;
            }
        };

        // Compute the stake for the leader certificate.
        let (stake_with_leader, stake_without_leader) =
            self.compute_stake_for_leader_certificate(leader_certificate_id, current_certificates, &previous_committee);
        // Return 'true' if any of the following conditions hold:
        stake_with_leader >= previous_committee.availability_threshold()
            || stake_without_leader >= previous_committee.quorum_threshold()
            || self.is_timer_expired()
    }

    /// Computes the amount of stake that has & has not signed for the leader certificate.
    fn compute_stake_for_leader_certificate(
        &self,
        leader_certificate_id: Field<N>,
        current_certificates: IndexSet<BatchCertificate<N>>,
        current_committee: &Committee<N>,
    ) -> (u64, u64) {
        // If there are no current certificates, return early.
        if current_certificates.is_empty() {
            return (0, 0);
        }

        // Initialize a tracker for the stake with the leader.
        let mut stake_with_leader = 0u64;
        // Initialize a tracker for the stake without the leader.
        let mut stake_without_leader = 0u64;
        // Iterate over the current certificates.
        for certificate in current_certificates {
            // Retrieve the stake for the author of the certificate.
            let stake = current_committee.get_stake(certificate.author());
            // Determine if the certificate includes the leader.
            match certificate.previous_certificate_ids().iter().any(|id| *id == leader_certificate_id) {
                // If the certificate includes the leader, add the stake to the stake with the leader.
                true => stake_with_leader = stake_with_leader.saturating_add(stake),
                // If the certificate does not include the leader, add the stake to the stake without the leader.
                false => stake_without_leader = stake_without_leader.saturating_add(stake),
            }
        }
        // Return the stake with the leader, and the stake without the leader.
        (stake_with_leader, stake_without_leader)
    }
}

impl<N: Network> BFT<N> {
    /// Stores the certificate in the DAG, and attempts to commit one or more anchors.
    async fn update_dag<const ALLOW_LEDGER_ACCESS: bool>(&self, certificate: BatchCertificate<N>) -> Result<()> {
        // Acquire the BFT lock.
        let _lock = self.lock.lock().await;

        // Retrieve the certificate round.
        let certificate_round = certificate.round();
        // Insert the certificate into the DAG.
        self.dag.write().insert(certificate);

        // Construct the commit round.
        let commit_round = certificate_round.saturating_sub(1);
        // If the commit round is odd, return early.
        if commit_round % 2 != 0 || commit_round < 2 {
            return Ok(());
        }
        // If the commit round is at or below the last committed round, return early.
        if commit_round <= self.dag.read().last_committed_round() {
            return Ok(());
        }

        // Retrieve the previous committee for the commit round.
        let Ok(previous_committee) = self.ledger().get_previous_committee_for_round(commit_round) else {
            bail!("BFT failed to retrieve the committee for commit round {commit_round}");
        };
        // Compute the leader for the commit round.
        let Ok(leader) = previous_committee.get_leader(commit_round) else {
            bail!("BFT failed to compute the leader for commit round {commit_round}");
        };
        // Retrieve the leader certificate for the commit round.
        let Some(leader_certificate) = self.dag.read().get_certificate_for_round_with_author(commit_round, leader)
        else {
            trace!("BFT did not find the leader certificate for commit round {commit_round} yet");
            return Ok(());
        };
        // Retrieve all of the certificates for the **certificate** round.
        let Some(certificates) = self.dag.read().get_certificates_for_round(certificate_round) else {
            // TODO (howardwu): Investigate how many certificates we should have at this point.
            bail!("BFT failed to retrieve the certificates for certificate round {certificate_round}");
        };
        // Construct a set over the authors who included the leader's certificate in the certificate round.
        let authors = certificates
            .values()
            .filter_map(|c| match c.previous_certificate_ids().contains(&leader_certificate.certificate_id()) {
                true => Some(c.author()),
                false => None,
            })
            .collect();
        // Check if the leader is ready to be committed.
        if !previous_committee.is_availability_threshold_reached(&authors) {
            // If the leader is not ready to be committed, return early.
            trace!("BFT is not ready to commit {commit_round}");
            return Ok(());
        }

        /* Proceeding to commit the leader. */

        // Commit the leader certificate, and all previous leader certificates since the last committed round.
        self.commit_leader_certificate::<ALLOW_LEDGER_ACCESS, false>(leader_certificate).await
    }

    /// Commits the leader certificate, and all previous leader certificates since the last committed round.
    async fn commit_leader_certificate<const ALLOW_LEDGER_ACCESS: bool, const IS_SYNCING: bool>(
        &self,
        leader_certificate: BatchCertificate<N>,
    ) -> Result<()> {
        // Retrieve the leader certificate round.
        let leader_round = leader_certificate.round();
        // Compute the commit subdag.
        let commit_subdag = match self.order_dag_with_dfs::<ALLOW_LEDGER_ACCESS>(leader_certificate) {
            Ok(subdag) => subdag,
            Err(e) => bail!("BFT failed to order the DAG with DFS - {e}"),
        };
        // Initialize a map for the deduped transmissions.
        let mut transmissions = IndexMap::new();
        // Start from the oldest leader certificate.
        for certificate in commit_subdag.values().flatten() {
            // Update the DAG.
            if IS_SYNCING {
                self.dag.write().commit(certificate, self.storage().max_gc_rounds());
            }
            // Retrieve the transmissions.
            for transmission_id in certificate.transmission_ids() {
                // If the transmission already exists in the map, skip it.
                if transmissions.contains_key(transmission_id) {
                    continue;
                }
                // If the transmission already exists in the ledger, skip it.
                // Note: On failure to read from the ledger, we skip including this transmission, out of safety.
                if self.ledger().contains_transmission(transmission_id).unwrap_or(true) {
                    continue;
                }
                // Retrieve the transmission.
                let Some(transmission) = self.storage().get_transmission(*transmission_id) else {
                    bail!("BFT failed to retrieve transmission {}", fmt_id(transmission_id));
                };
                // Add the transmission to the set.
                transmissions.insert(*transmission_id, transmission);
            }
        }
        // If the node is not syncing, trigger consensus, as this will build a new block for the ledger.
        if !IS_SYNCING {
            // Construct the subdag.
            let subdag = Subdag::from(commit_subdag.clone())?;
            // Retrieve the anchor round.
            let anchor_round = subdag.anchor_round();
            // Retrieve the number of transmissions.
            let num_transmissions = transmissions.len();
            // Retrieve metadata about the subdag.
            let subdag_metadata = subdag.iter().map(|(round, c)| (*round, c.len())).collect::<Vec<_>>();

            // Ensure the subdag anchor round matches the leader round.
            ensure!(
                anchor_round == leader_round,
                "BFT failed to commit - the subdag anchor round {anchor_round} does not match the leader round {leader_round}",
            );

            // Trigger consensus.
            if let Some(consensus_sender) = self.consensus_sender.get() {
                // Initialize a callback sender and receiver.
                let (callback_sender, callback_receiver) = oneshot::channel();
                // Send the subdag and transmissions to consensus.
                consensus_sender.tx_consensus_subdag.send((subdag, transmissions, callback_sender)).await?;
                // Await the callback to continue.
                match callback_receiver.await {
                    Ok(Ok(())) => (), // continue
                    Ok(Err(e)) => {
                        error!("BFT failed to advance the subdag for round {anchor_round} - {e}");
                        return Ok(());
                    }
                    Err(e) => {
                        error!("BFT failed to receive the callback for round {anchor_round} - {e}");
                        return Ok(());
                    }
                }
            }

            info!(
                "\n\nCommitting a subdag from round {anchor_round} with {num_transmissions} transmissions: {subdag_metadata:?}\n"
            );
            // Update the DAG, as the subdag was successfully included into a block.
            let mut dag_write = self.dag.write();
            for certificate in commit_subdag.values().flatten() {
                dag_write.commit(certificate, self.storage().max_gc_rounds());
            }
        }
        Ok(())
    }

    /// Returns the subdag of batch certificates to commit.
    fn order_dag_with_dfs<const ALLOW_LEDGER_ACCESS: bool>(
        &self,
        leader_certificate: BatchCertificate<N>,
    ) -> Result<BTreeMap<u64, IndexSet<BatchCertificate<N>>>> {
        // Initialize a map for the certificates to commit.
        let mut commit = BTreeMap::<u64, IndexSet<_>>::new();
        // Initialize a set for the already ordered certificates.
        let mut already_ordered = HashSet::new();
        // Initialize a buffer for the certificates to order.
        let mut buffer = vec![leader_certificate];
        // Iterate over the certificates to order.
        while let Some(certificate) = buffer.pop() {
            // Insert the certificate into the map.
            commit.entry(certificate.round()).or_default().insert(certificate.clone());

            // Check if the previous certificate is below the GC round.
            let previous_round = certificate.round().saturating_sub(1);
            if previous_round + self.storage().max_gc_rounds() <= self.dag.read().last_committed_round() {
                continue;
            }
            // Iterate over the previous certificate IDs.
            // Note: Using '.rev()' ensures we remain order-preserving (i.e. "left-to-right" on each level),
            // because this 'while' loop uses 'pop()' to retrieve the next certificate to order.
            for previous_certificate_id in certificate.previous_certificate_ids().iter().rev() {
                // If the previous certificate is already ordered, continue.
                if already_ordered.contains(previous_certificate_id) {
                    continue;
                }
                // If the previous certificate was recently committed, continue.
                if self.dag.read().is_recently_committed(previous_round, *previous_certificate_id) {
                    continue;
                }
                // Retrieve the previous certificate.
                let previous_certificate = {
                    // Start by retrieving the previous certificate from the DAG.
                    match self.dag.read().get_certificate_for_round_with_id(previous_round, *previous_certificate_id) {
                        // If the previous certificate is found, return it.
                        Some(previous_certificate) => previous_certificate,
                        // If the previous certificate is not found, retrieve it from the storage.
                        None => match self.storage().get_certificate(*previous_certificate_id) {
                            // If the previous certificate is found, return it.
                            Some(previous_certificate) => previous_certificate,
                            // Otherwise, retrieve the previous certificate from the ledger.
                            None => {
                                if ALLOW_LEDGER_ACCESS {
                                    match self.ledger().get_batch_certificate(previous_certificate_id) {
                                        // If the previous certificate is found, return it.
                                        Ok(previous_certificate) => previous_certificate,
                                        // Otherwise, the previous certificate is missing, and throw an error.
                                        Err(e) => {
                                            bail!(
                                                "Missing previous certificate {} for round {previous_round} - {e}",
                                                fmt_id(previous_certificate_id)
                                            )
                                        }
                                    }
                                } else {
                                    // Otherwise, the previous certificate is missing, and throw an error.
                                    bail!(
                                        "Missing previous certificate {} for round {previous_round}",
                                        fmt_id(previous_certificate_id)
                                    )
                                }
                            }
                        },
                    }
                };
                // Insert the previous certificate into the set of already ordered certificates.
                already_ordered.insert(previous_certificate.certificate_id());
                // Insert the previous certificate into the buffer.
                buffer.push(previous_certificate);
            }
        }
        // Ensure we only retain certificates that are above the GC round.
        commit.retain(|round, _| round + self.storage().max_gc_rounds() > self.dag.read().last_committed_round());
        // Return the certificates to commit.
        Ok(commit)
    }
}

impl<N: Network> BFT<N> {
    /// Starts the BFT handlers.
    fn start_handlers(&self, bft_receiver: BFTReceiver<N>) {
        let BFTReceiver {
            mut rx_primary_round,
            mut rx_primary_certificate,
            mut rx_sync_bft_dag_at_bootup,
            mut rx_sync_bft,
        } = bft_receiver;

        // Process the current round from the primary.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((current_round, callback)) = rx_primary_round.recv().await {
                callback.send(self_.update_to_next_round(current_round)).ok();
            }
        });

        // Process the certificate from the primary.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((certificate, callback)) = rx_primary_certificate.recv().await {
                // Update the DAG with the certificate.
                let result = self_.update_dag::<true>(certificate).await;
                // Send the callback **after** updating the DAG.
                // Note: We must await the DAG update before proceeding.
                callback.send(result).ok();
            }
        });

        // Process the request to sync the BFT DAG at bootup.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((leader_certificates, certificates)) = rx_sync_bft_dag_at_bootup.recv().await {
                self_.sync_bft_dag_at_bootup(leader_certificates, certificates).await;
            }
        });

        // Process the request to sync the BFT.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((certificate, callback)) = rx_sync_bft.recv().await {
                // Update the DAG with the certificate.
                let result = self_.update_dag::<true>(certificate).await;
                // Send the callback **after** updating the DAG.
                // Note: We must await the DAG update before proceeding.
                callback.send(result).ok();
            }
        });
    }

    /// Syncs the BFT DAG with the given leader certificates and batch certificates.
    ///
    /// This method starts by inserting all certificates (except the latest leader certificate)
    /// into the DAG. Then, it commits all leader certificates (except the latest leader certificate).
    /// Finally, it updates the DAG with the latest leader certificate.
    async fn sync_bft_dag_at_bootup(
        &self,
        leader_certificates: Vec<BatchCertificate<N>>,
        certificates: Vec<BatchCertificate<N>>,
    ) {
        // Split the leader certificates into past leader certificates and the latest leader certificate.
        let (past_leader_certificates, leader_certificate) = {
            // Compute the penultimate index.
            let index = leader_certificates.len().saturating_sub(1);
            // Split the leader certificates.
            let (past, latest) = leader_certificates.split_at(index);
            debug_assert!(latest.len() == 1, "There should only be one latest leader certificate");
            // Retrieve the latest leader certificate.
            match latest.first() {
                Some(leader_certificate) => (past, leader_certificate.clone()),
                // If there is no latest leader certificate, return early.
                None => return,
            }
        };
        {
            // Acquire the BFT write lock.
            let mut dag = self.dag.write();
            // Iterate over the certificates.
            for certificate in certificates {
                // If the certificate is not the latest leader certificate, insert it.
                if leader_certificate.certificate_id() != certificate.certificate_id() {
                    // Insert the certificate into the DAG.
                    dag.insert(certificate);
                }
            }
            // Iterate over the leader certificates.
            for leader_certificate in past_leader_certificates {
                // Commit the leader certificate.
                dag.commit(leader_certificate, self.storage().max_gc_rounds());
            }
        }
        // Commit the latest leader certificate.
        if let Err(e) = self.commit_leader_certificate::<true, true>(leader_certificate).await {
            error!("BFT failed to update the DAG with the latest leader certificate - {e}");
        }
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the BFT.
    pub async fn shut_down(&self) {
        info!("Shutting down the BFT...");
        // Acquire the lock.
        let _lock = self.lock.lock().await;
        // Shut down the primary.
        self.primary.shut_down().await;
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        helpers::{now, Storage},
        BFT,
    };
    use snarkos_account::Account;
    use snarkos_node_bft_ledger_service::MockLedgerService;
    use snarkvm::{
        ledger::narwhal::batch_certificate::test_helpers::{
            sample_batch_certificate,
            sample_batch_certificate_for_round,
        },
        utilities::TestRng,
    };

    use anyhow::Result;
    use indexmap::IndexSet;
    use std::sync::{atomic::Ordering, Arc};

    #[test]
    #[tracing_test::traced_test]
    fn test_is_leader_quorum_odd() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee(rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));
        let storage = Storage::new(ledger.clone(), 10);

        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;
        assert!(bft.is_timer_expired()); // 0 + 5 < now()

        // Ensure this call succeeds on an odd round.
        let result = bft.is_leader_quorum_or_nonleaders_available(1);
        assert!(result); // no previous leader certificate

        // Set the leader certificate.
        let leader_certificate = sample_batch_certificate(rng);
        *bft.leader_certificate.write() = Some(leader_certificate);

        // Ensure this call succeeds on an odd round.
        let result = bft.is_leader_quorum_or_nonleaders_available(1);
        assert!(result); // should now fall through to end of function

        // Set the timer to now().
        bft.leader_certificate_timer.store(now(), Ordering::SeqCst);
        assert!(!bft.is_timer_expired());

        // Ensure this call succeeds on an odd round.
        let result = bft.is_leader_quorum_or_nonleaders_available(1);
        // Should now return false, as the timer is not expired.
        assert!(!result); // should now fall through to end of function
        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_is_leader_quorum_even_out_of_sync() -> Result<()> {
        let rng = &mut TestRng::default();

        // Create a committee with round 1.
        let committee = snarkvm::ledger::committee::test_helpers::sample_committee(rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));
        let storage = Storage::new(ledger.clone(), 10);

        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;
        assert!(bft.is_timer_expired()); // 0 + 5 < now()

        // Store is at round 1, and we are checking for round 2.
        // Ensure this call fails on an even round.
        let result = bft.is_leader_quorum_or_nonleaders_available(2);
        assert!(!result);
        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_is_leader_quorum_even() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round(2, rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));
        let storage = Storage::new(ledger.clone(), 10);
        assert_eq!(storage.current_round(), 2);

        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;
        assert!(bft.is_timer_expired()); // 0 + 5 < now()

        // Ensure this call fails on an even round.
        let result = bft.is_leader_quorum_or_nonleaders_available(2);
        assert!(!result);
        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_is_even_round_ready() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round(2, rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee.clone()));
        let storage = Storage::new(ledger.clone(), 10);

        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;

        let result = bft.is_even_round_ready_for_next_round(IndexSet::new(), committee.clone(), 2);
        assert!(!result);

        // Set the leader certificate.
        let leader_certificate = sample_batch_certificate_for_round(2, rng);
        *bft.leader_certificate.write() = Some(leader_certificate);

        let result = bft.is_even_round_ready_for_next_round(IndexSet::new(), committee, 2);
        // If leader certificate is set, we should be ready for next round.
        assert!(result);
        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_update_leader_certificate_odd() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee(rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));
        let storage = Storage::new(ledger.clone(), 10);

        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;

        // Ensure this call fails on an odd round.
        let result = bft.update_leader_certificate_to_even_round(1);
        assert!(!result);
        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_update_leader_certificate_bad_round() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee(rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));
        let storage = Storage::new(ledger.clone(), 10);

        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;

        // Ensure this call succeeds on an even round.
        let result = bft.update_leader_certificate_to_even_round(6);
        assert!(!result);
        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_update_leader_certificate_even() -> Result<()> {
        let rng = &mut TestRng::default();

        // Set the current round.
        let current_round = 3;

        // Sample the certificates.
        let (_, certificates) = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_with_previous_certificates(
            current_round,
            rng,
        );

        // Initialize the committee.
        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round_and_members(
            2,
            vec![
                certificates[0].author(),
                certificates[1].author(),
                certificates[2].author(),
                certificates[3].author(),
            ],
            rng,
        );

        // Initialize the ledger.
        let ledger = Arc::new(MockLedgerService::new(committee.clone()));

        // Initialize the storage.
        let storage = Storage::new(ledger.clone(), 10);
        storage.testing_only_insert_certificate_testing_only(certificates[0].clone());
        storage.testing_only_insert_certificate_testing_only(certificates[1].clone());
        storage.testing_only_insert_certificate_testing_only(certificates[2].clone());
        storage.testing_only_insert_certificate_testing_only(certificates[3].clone());
        assert_eq!(storage.current_round(), 2);

        // Retrieve the leader certificate.
        let leader = committee.get_leader(2).unwrap();
        let leader_certificate = storage.get_certificate_for_round_with_author(2, leader).unwrap();

        // Initialize the BFT.
        let account = Account::new(rng)?;
        let bft = BFT::new(account, storage.clone(), ledger, None, &[], None)?;

        // Set the leader certificate.
        *bft.leader_certificate.write() = Some(leader_certificate);

        // Update the leader certificate.
        // Ensure this call succeeds on an even round.
        let result = bft.update_leader_certificate_to_even_round(2);
        assert!(result);

        Ok(())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_order_dag_with_dfs() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round(1, rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));

        // Initialize the round parameters.
        let previous_round = 2; // <- This must be an even number, for `BFT::update_dag` to behave correctly below.
        let current_round = previous_round + 1;

        // Sample the current certificate and previous certificates.
        let (certificate, previous_certificates) = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_with_previous_certificates(
            current_round,
            rng,
        );

        /* Test GC */

        // Ensure the function succeeds in returning only certificates above GC.
        {
            // Initialize the storage.
            let storage = Storage::new(ledger.clone(), 1);
            // Initialize the BFT.
            let bft = BFT::new(account.clone(), storage, ledger.clone(), None, &[], None)?;

            // Insert a mock DAG in the BFT.
            *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(3);

            // Insert the previous certificates into the BFT.
            for certificate in previous_certificates.clone() {
                assert!(bft.update_dag::<false>(certificate).await.is_ok());
            }

            // Ensure this call succeeds and returns all given certificates.
            let result = bft.order_dag_with_dfs::<false>(certificate.clone());
            assert!(result.is_ok());
            let candidate_certificates = result.unwrap().into_values().flatten().collect::<Vec<_>>();
            assert_eq!(candidate_certificates.len(), 1);
            let expected_certificates = vec![certificate.clone()];
            assert_eq!(
                candidate_certificates.iter().map(|c| c.certificate_id()).collect::<Vec<_>>(),
                expected_certificates.iter().map(|c| c.certificate_id()).collect::<Vec<_>>()
            );
            assert_eq!(candidate_certificates, expected_certificates);
        }

        /* Test normal case */

        // Ensure the function succeeds in returning all given certificates.
        {
            // Initialize the storage.
            let storage = Storage::new(ledger.clone(), 1);
            // Initialize the BFT.
            let bft = BFT::new(account, storage, ledger, None, &[], None)?;

            // Insert a mock DAG in the BFT.
            *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(2);

            // Insert the previous certificates into the BFT.
            for certificate in previous_certificates.clone() {
                assert!(bft.update_dag::<false>(certificate).await.is_ok());
            }

            // Ensure this call succeeds and returns all given certificates.
            let result = bft.order_dag_with_dfs::<false>(certificate.clone());
            assert!(result.is_ok());
            let candidate_certificates = result.unwrap().into_values().flatten().collect::<Vec<_>>();
            assert_eq!(candidate_certificates.len(), 5);
            let expected_certificates = vec![
                previous_certificates[0].clone(),
                previous_certificates[1].clone(),
                previous_certificates[2].clone(),
                previous_certificates[3].clone(),
                certificate,
            ];
            assert_eq!(
                candidate_certificates.iter().map(|c| c.certificate_id()).collect::<Vec<_>>(),
                expected_certificates.iter().map(|c| c.certificate_id()).collect::<Vec<_>>()
            );
            assert_eq!(candidate_certificates, expected_certificates);
        }

        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_order_dag_with_dfs_fails_on_missing_previous_certificate() -> Result<()> {
        let rng = &mut TestRng::default();

        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round(1, rng);
        let account = Account::new(rng)?;
        let ledger = Arc::new(MockLedgerService::new(committee));

        // Initialize the round parameters.
        let previous_round = 2; // <- This must be an even number, for `BFT::update_dag` to behave correctly below.
        let current_round = previous_round + 1;

        // Sample the current certificate and previous certificates.
        let (certificate, previous_certificates) = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_with_previous_certificates(
            current_round,
            rng,
        );
        // Construct the previous certificate IDs.
        let previous_certificate_ids: IndexSet<_> = previous_certificates.iter().map(|c| c.certificate_id()).collect();

        /* Test missing previous certificate. */

        // Initialize the storage.
        let storage = Storage::new(ledger.clone(), 1);
        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger, None, &[], None)?;

        // The expected error message.
        let error_msg = format!(
            "Missing previous certificate {} for round {previous_round}",
            crate::helpers::fmt_id(previous_certificate_ids[3]),
        );

        // Ensure this call fails on a missing previous certificate.
        let result = bft.order_dag_with_dfs::<false>(certificate);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), error_msg);
        Ok(())
    }
}
