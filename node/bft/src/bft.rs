// Copyright 2024 Aleo Network Foundation
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
    MAX_LEADER_CERTIFICATE_DELAY_IN_SECS,
};
use snarkos_account::Account;
use snarkos_node_bft_ledger_service::LedgerService;
use snarkvm::{
    console::account::Address,
    ledger::{
        block::Transaction,
        committee::Committee,
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
        puzzle::{Solution, SolutionID},
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

    /// Returns `true` if the primary is synced.
    pub fn is_synced(&self) -> bool {
        self.primary.is_synced()
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
    /// Returns the worker transmission IDs.
    pub fn worker_transmission_ids(&self) -> impl '_ + Iterator<Item = TransmissionID<N>> {
        self.primary.worker_transmission_ids()
    }

    /// Returns the worker transmissions.
    pub fn worker_transmissions(&self) -> impl '_ + Iterator<Item = (TransmissionID<N>, Transmission<N>)> {
        self.primary.worker_transmissions()
    }

    /// Returns the worker solutions.
    pub fn worker_solutions(&self) -> impl '_ + Iterator<Item = (SolutionID<N>, Data<Solution<N>>)> {
        self.primary.worker_solutions()
    }

    /// Returns the worker transactions.
    pub fn worker_transactions(&self) -> impl '_ + Iterator<Item = (N::TransactionID, Data<Transaction<N>>)> {
        self.primary.worker_transactions()
    }
}

impl<N: Network> BFT<N> {
    /// Stores the certificate in the DAG, and attempts to commit one or more anchors.
    fn update_to_next_round(&self, current_round: u64) -> bool {
        // Ensure the current round is at least the storage round (this is a sanity check).
        let storage_round = self.storage().current_round();
        if current_round < storage_round {
            debug!(
                "BFT is safely skipping an update for round {current_round}, as storage is at round {storage_round}"
            );
            return false;
        }

        // Determine if the BFT is ready to update to the next round.
        let is_ready = match current_round % 2 == 0 {
            true => self.update_leader_certificate_to_even_round(current_round),
            false => self.is_leader_quorum_or_nonleaders_available(current_round),
        };

        #[cfg(feature = "metrics")]
        {
            let start = self.leader_certificate_timer.load(Ordering::SeqCst);
            // Only log if the timer was set, otherwise we get a time difference since the EPOCH.
            if start > 0 {
                let end = now();
                let elapsed = std::time::Duration::from_secs((end - start) as u64);
                metrics::histogram(metrics::bft::COMMIT_ROUNDS_LATENCY, elapsed.as_secs_f64());
            }
        }

        // Log whether the round is going to update.
        if current_round % 2 == 0 {
            // Determine if there is a leader certificate.
            if let Some(leader_certificate) = self.leader_certificate.read().as_ref() {
                // Ensure the state of the leader certificate is consistent with the BFT being ready.
                if !is_ready {
                    trace!(is_ready, "BFT - A leader certificate was found, but 'is_ready' is false");
                }
                // Log the leader election.
                let leader_round = leader_certificate.round();
                match leader_round == current_round {
                    true => {
                        info!("\n\nRound {current_round} elected a leader - {}\n", leader_certificate.author());
                        #[cfg(feature = "metrics")]
                        metrics::increment_counter(metrics::bft::LEADERS_ELECTED);
                    }
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
                return false;
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

        // Retrieve the committee lookback of the current round.
        let committee_lookback = match self.ledger().get_committee_lookback_for_round(current_round) {
            Ok(committee) => committee,
            Err(e) => {
                error!("BFT failed to retrieve the committee lookback for the even round {current_round} - {e}");
                return false;
            }
        };
        // Determine the leader of the current round.
        let leader = match self.ledger().latest_leader() {
            Some((cached_round, cached_leader)) if cached_round == current_round => cached_leader,
            _ => {
                // Compute the leader for the current round.
                let computed_leader = match committee_lookback.get_leader(current_round) {
                    Ok(leader) => leader,
                    Err(e) => {
                        error!("BFT failed to compute the leader for the even round {current_round} - {e}");
                        return false;
                    }
                };

                // Cache the computed leader.
                self.ledger().update_latest_leader(current_round, computed_leader);

                computed_leader
            }
        };
        // Find and set the leader certificate, if the leader was present in the current even round.
        let leader_certificate = current_certificates.iter().find(|certificate| certificate.author() == leader);
        *self.leader_certificate.write() = leader_certificate.cloned();

        self.is_even_round_ready_for_next_round(current_certificates, committee_lookback, current_round)
    }

    /// Returns 'true' if the quorum threshold `(2f + 1)` is reached for this round under one of the following conditions:
    ///  - If the leader certificate is set for the current even round.
    ///  - The timer for the leader certificate has expired.
    fn is_even_round_ready_for_next_round(
        &self,
        certificates: IndexSet<BatchCertificate<N>>,
        committee: Committee<N>,
        current_round: u64,
    ) -> bool {
        // Retrieve the authors for the current round.
        let authors = certificates.into_iter().map(|c| c.author()).collect();
        // Check if quorum threshold is reached.
        if !committee.is_quorum_threshold_reached(&authors) {
            trace!("BFT failed to reach quorum threshold in even round {current_round}");
            return false;
        }
        // If the leader certificate is set for the current even round, return 'true'.
        if let Some(leader_certificate) = self.leader_certificate.read().as_ref() {
            if leader_certificate.round() == current_round {
                return true;
            }
        }
        // If the timer has expired, and we can achieve quorum threshold (2f + 1) without the leader, return 'true'.
        if self.is_timer_expired() {
            debug!("BFT (timer expired) - Advancing from round {current_round} to the next round (without the leader)");
            return true;
        }
        // Otherwise, return 'false'.
        false
    }

    /// Returns `true` if the timer for the leader certificate has expired.
    fn is_timer_expired(&self) -> bool {
        self.leader_certificate_timer.load(Ordering::SeqCst) + MAX_LEADER_CERTIFICATE_DELAY_IN_SECS <= now()
    }

    /// Returns 'true' if the quorum threshold `(2f + 1)` is reached for this round under one of the following conditions:
    ///  - The leader certificate is `None`.
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
        // Retrieve the certificates for the current round.
        let current_certificates = self.storage().get_certificates_for_round(current_round);
        // Retrieve the committee lookback for the current round.
        let committee_lookback = match self.ledger().get_committee_lookback_for_round(current_round) {
            Ok(committee) => committee,
            Err(e) => {
                error!("BFT failed to retrieve the committee lookback for the odd round {current_round} - {e}");
                return false;
            }
        };
        // Retrieve the authors of the current certificates.
        let authors = current_certificates.clone().into_iter().map(|c| c.author()).collect();
        // Check if quorum threshold is reached.
        if !committee_lookback.is_quorum_threshold_reached(&authors) {
            trace!("BFT failed reach quorum threshold in odd round {current_round}. ");
            return false;
        }
        // Retrieve the leader certificate.
        let Some(leader_certificate) = self.leader_certificate.read().clone() else {
            // If there is no leader certificate for the previous round, return 'true'.
            return true;
        };
        // Compute the stake for the leader certificate.
        let (stake_with_leader, stake_without_leader) = self.compute_stake_for_leader_certificate(
            leader_certificate.id(),
            current_certificates,
            &committee_lookback,
        );
        // Return 'true' if any of the following conditions hold:
        stake_with_leader >= committee_lookback.availability_threshold()
            || stake_without_leader >= committee_lookback.quorum_threshold()
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
    async fn update_dag<const ALLOW_LEDGER_ACCESS: bool, const IS_SYNCING: bool>(
        &self,
        certificate: BatchCertificate<N>,
    ) -> Result<()> {
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

        /* Proceeding to check if the leader is ready to be committed. */
        trace!("Checking if the leader is ready to be committed for round {commit_round}...");

        // Retrieve the committee lookback for the commit round.
        let Ok(committee_lookback) = self.ledger().get_committee_lookback_for_round(commit_round) else {
            bail!("BFT failed to retrieve the committee with lag for commit round {commit_round}");
        };

        // Either retrieve the cached leader or compute it.
        let leader = match self.ledger().latest_leader() {
            Some((cached_round, cached_leader)) if cached_round == commit_round => cached_leader,
            _ => {
                // Compute the leader for the commit round.
                let Ok(computed_leader) = committee_lookback.get_leader(commit_round) else {
                    bail!("BFT failed to compute the leader for commit round {commit_round}");
                };

                // Cache the computed leader.
                self.ledger().update_latest_leader(commit_round, computed_leader);

                computed_leader
            }
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
            .filter_map(|c| match c.previous_certificate_ids().contains(&leader_certificate.id()) {
                true => Some(c.author()),
                false => None,
            })
            .collect();
        // Check if the leader is ready to be committed.
        if !committee_lookback.is_availability_threshold_reached(&authors) {
            // If the leader is not ready to be committed, return early.
            trace!("BFT is not ready to commit {commit_round}");
            return Ok(());
        }

        /* Proceeding to commit the leader. */
        info!("Proceeding to commit round {commit_round} with leader '{}'", fmt_id(leader));

        // Commit the leader certificate, and all previous leader certificates since the last committed round.
        self.commit_leader_certificate::<ALLOW_LEDGER_ACCESS, IS_SYNCING>(leader_certificate).await
    }

    /// Commits the leader certificate, and all previous leader certificates since the last committed round.
    async fn commit_leader_certificate<const ALLOW_LEDGER_ACCESS: bool, const IS_SYNCING: bool>(
        &self,
        leader_certificate: BatchCertificate<N>,
    ) -> Result<()> {
        // Fetch the leader round.
        let latest_leader_round = leader_certificate.round();
        // Determine the list of all previous leader certificates since the last committed round.
        // The order of the leader certificates is from **newest** to **oldest**.
        let mut leader_certificates = vec![leader_certificate.clone()];
        {
            // Retrieve the leader round.
            let leader_round = leader_certificate.round();

            let mut current_certificate = leader_certificate;
            for round in (self.dag.read().last_committed_round() + 2..=leader_round.saturating_sub(2)).rev().step_by(2)
            {
                // Retrieve the previous committee for the leader round.
                let previous_committee_lookback = match self.ledger().get_committee_lookback_for_round(round) {
                    Ok(committee) => committee,
                    Err(e) => {
                        bail!("BFT failed to retrieve a previous committee lookback for the even round {round} - {e}");
                    }
                };
                // Either retrieve the cached leader or compute it.
                let leader = match self.ledger().latest_leader() {
                    Some((cached_round, cached_leader)) if cached_round == round => cached_leader,
                    _ => {
                        // Compute the leader for the commit round.
                        let computed_leader = match previous_committee_lookback.get_leader(round) {
                            Ok(leader) => leader,
                            Err(e) => {
                                bail!("BFT failed to compute the leader for the even round {round} - {e}");
                            }
                        };

                        // Cache the computed leader.
                        self.ledger().update_latest_leader(round, computed_leader);

                        computed_leader
                    }
                };
                // Retrieve the previous leader certificate.
                let Some(previous_certificate) = self.dag.read().get_certificate_for_round_with_author(round, leader)
                else {
                    continue;
                };
                // Determine if there is a path between the previous certificate and the current certificate.
                if self.is_linked(previous_certificate.clone(), current_certificate.clone())? {
                    // Add the previous leader certificate to the list of certificates to commit.
                    leader_certificates.push(previous_certificate.clone());
                    // Update the current certificate to the previous leader certificate.
                    current_certificate = previous_certificate;
                }
            }
        }

        // Iterate over the leader certificates to commit.
        for leader_certificate in leader_certificates.into_iter().rev() {
            // Retrieve the leader certificate round.
            let leader_round = leader_certificate.round();
            // Compute the commit subdag.
            let commit_subdag = match self.order_dag_with_dfs::<ALLOW_LEDGER_ACCESS>(leader_certificate) {
                Ok(subdag) => subdag,
                Err(e) => bail!("BFT failed to order the DAG with DFS - {e}"),
            };
            // If the node is not syncing, trigger consensus, as this will build a new block for the ledger.
            if !IS_SYNCING {
                // Initialize a map for the deduped transmissions.
                let mut transmissions = IndexMap::new();
                // Initialize a map for the deduped transaction ids.
                let mut seen_transaction_ids = IndexSet::new();
                // Initialize a map for the deduped solution ids.
                let mut seen_solution_ids = IndexSet::new();
                // Start from the oldest leader certificate.
                for certificate in commit_subdag.values().flatten() {
                    // Retrieve the transmissions.
                    for transmission_id in certificate.transmission_ids() {
                        // If the transaction ID or solution ID already exists in the map, skip it.
                        // Note: This additional check is done to ensure that we do not include duplicate
                        // transaction IDs or solution IDs that may have a different transmission ID.
                        match transmission_id {
                            TransmissionID::Solution(solution_id, _) => {
                                // If the solution already exists, skip it.
                                if seen_solution_ids.contains(&solution_id) {
                                    continue;
                                }
                            }
                            TransmissionID::Transaction(transaction_id, _) => {
                                // If the transaction already exists, skip it.
                                if seen_transaction_ids.contains(transaction_id) {
                                    continue;
                                }
                            }
                            TransmissionID::Ratification => {
                                bail!("Ratifications are currently not supported in the BFT.")
                            }
                        }
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
                            bail!(
                                "BFT failed to retrieve transmission '{}.{}' from round {}",
                                fmt_id(transmission_id),
                                fmt_id(transmission_id.checksum().unwrap_or_default()).dimmed(),
                                certificate.round()
                            );
                        };
                        // Insert the transaction ID or solution ID into the map.
                        match transmission_id {
                            TransmissionID::Solution(id, _) => {
                                seen_solution_ids.insert(id);
                            }
                            TransmissionID::Transaction(id, _) => {
                                seen_transaction_ids.insert(id);
                            }
                            TransmissionID::Ratification => {}
                        }
                        // Add the transmission to the set.
                        transmissions.insert(*transmission_id, transmission);
                    }
                }
                // Trigger consensus, as this will build a new block for the ledger.
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
            }

            // Update the DAG, as the subdag was successfully included into a block.
            let mut dag_write = self.dag.write();
            for certificate in commit_subdag.values().flatten() {
                dag_write.commit(certificate, self.storage().max_gc_rounds());
            }
        }

        // Perform garbage collection based on the latest committed leader round.
        self.storage().garbage_collect_certificates(latest_leader_round);

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
                // If the previous certificate already exists in the ledger, continue.
                if ALLOW_LEDGER_ACCESS && self.ledger().contains_certificate(previous_certificate_id).unwrap_or(false) {
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
                            // Otherwise, the previous certificate is missing, and throw an error.
                            None => bail!(
                                "Missing previous certificate {} for round {previous_round}",
                                fmt_id(previous_certificate_id)
                            ),
                        },
                    }
                };
                // Insert the previous certificate into the set of already ordered certificates.
                already_ordered.insert(previous_certificate.id());
                // Insert the previous certificate into the buffer.
                buffer.push(previous_certificate);
            }
        }
        // Ensure we only retain certificates that are above the GC round.
        commit.retain(|round, _| round + self.storage().max_gc_rounds() > self.dag.read().last_committed_round());
        // Return the certificates to commit.
        Ok(commit)
    }

    /// Returns `true` if there is a path from the previous certificate to the current certificate.
    fn is_linked(
        &self,
        previous_certificate: BatchCertificate<N>,
        current_certificate: BatchCertificate<N>,
    ) -> Result<bool> {
        // Initialize the list containing the traversal.
        let mut traversal = vec![current_certificate.clone()];
        // Iterate over the rounds from the current certificate to the previous certificate.
        for round in (previous_certificate.round()..current_certificate.round()).rev() {
            // Retrieve all of the certificates for this past round.
            let Some(certificates) = self.dag.read().get_certificates_for_round(round) else {
                // This is a critical error, as the traversal should have these certificates.
                // If this error is hit, it is likely that the maximum GC rounds should be increased.
                bail!("BFT failed to retrieve the certificates for past round {round}");
            };
            // Filter the certificates to only include those that are in the traversal.
            traversal = certificates
                .into_values()
                .filter(|p| traversal.iter().any(|c| c.previous_certificate_ids().contains(&p.id())))
                .collect();
        }
        Ok(traversal.contains(&previous_certificate))
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
                let result = self_.update_dag::<true, false>(certificate).await;
                // Send the callback **after** updating the DAG.
                // Note: We must await the DAG update before proceeding.
                callback.send(result).ok();
            }
        });

        // Process the request to sync the BFT DAG at bootup.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some(certificates) = rx_sync_bft_dag_at_bootup.recv().await {
                self_.sync_bft_dag_at_bootup(certificates).await;
            }
        });

        // Process the request to sync the BFT.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((certificate, callback)) = rx_sync_bft.recv().await {
                // Update the DAG with the certificate.
                let result = self_.update_dag::<true, true>(certificate).await;
                // Send the callback **after** updating the DAG.
                // Note: We must await the DAG update before proceeding.
                callback.send(result).ok();
            }
        });
    }

    /// Syncs the BFT DAG with the given batch certificates. These batch certificates **must**
    /// already exist in the ledger.
    ///
    /// This method commits all the certificates into the DAG.
    /// Note that there is no need to insert the certificates into the DAG, because these certificates
    /// already exist in the ledger and therefore do not need to be re-ordered into future committed subdags.
    async fn sync_bft_dag_at_bootup(&self, certificates: Vec<BatchCertificate<N>>) {
        // Acquire the BFT write lock.
        let mut dag = self.dag.write();

        // Commit all the certificates excluding the latest leader certificate.
        for certificate in certificates {
            dag.commit(&certificate, self.storage().max_gc_rounds());
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
    use crate::{helpers::Storage, BFT, MAX_LEADER_CERTIFICATE_DELAY_IN_SECS};
    use snarkos_account::Account;
    use snarkos_node_bft_ledger_service::MockLedgerService;
    use snarkos_node_bft_storage_service::BFTMemoryService;
    use snarkvm::{
        console::account::{Address, PrivateKey},
        ledger::{
            committee::Committee,
            narwhal::batch_certificate::test_helpers::{sample_batch_certificate, sample_batch_certificate_for_round},
        },
        utilities::TestRng,
    };

    use anyhow::Result;
    use indexmap::{IndexMap, IndexSet};
    use std::sync::Arc;

    type CurrentNetwork = snarkvm::console::network::MainnetV0;

    /// Samples a new test instance, with an optional committee round and the given maximum GC rounds.
    fn sample_test_instance(
        committee_round: Option<u64>,
        max_gc_rounds: u64,
        rng: &mut TestRng,
    ) -> (
        Committee<CurrentNetwork>,
        Account<CurrentNetwork>,
        Arc<MockLedgerService<CurrentNetwork>>,
        Storage<CurrentNetwork>,
    ) {
        let committee = match committee_round {
            Some(round) => snarkvm::ledger::committee::test_helpers::sample_committee_for_round(round, rng),
            None => snarkvm::ledger::committee::test_helpers::sample_committee(rng),
        };
        let account = Account::new(rng).unwrap();
        let ledger = Arc::new(MockLedgerService::new(committee.clone()));
        let transmissions = Arc::new(BFTMemoryService::new());
        let storage = Storage::new(ledger.clone(), transmissions, max_gc_rounds);

        (committee, account, ledger, storage)
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_is_leader_quorum_odd() -> Result<()> {
        let rng = &mut TestRng::default();

        // Sample batch certificates.
        let mut certificates = IndexSet::new();
        certificates.insert(snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_for_round_with_previous_certificate_ids(1, IndexSet::new(), rng));
        certificates.insert(snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_for_round_with_previous_certificate_ids(1, IndexSet::new(), rng));
        certificates.insert(snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_for_round_with_previous_certificate_ids(1, IndexSet::new(), rng));
        certificates.insert(snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_for_round_with_previous_certificate_ids(1, IndexSet::new(), rng));

        // Initialize the committee.
        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round_and_members(
            1,
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
        let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), 10);
        // Initialize the account.
        let account = Account::new(rng)?;
        // Initialize the BFT.
        let bft = BFT::new(account.clone(), storage.clone(), ledger.clone(), None, &[], None)?;
        assert!(bft.is_timer_expired());
        // Ensure this call succeeds on an odd round.
        let result = bft.is_leader_quorum_or_nonleaders_available(1);
        // If timer has expired but quorum threshold is not reached, return 'false'.
        assert!(!result);
        // Insert certificates into storage.
        for certificate in certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }
        // Ensure this call succeeds on an odd round.
        let result = bft.is_leader_quorum_or_nonleaders_available(1);
        assert!(result); // no previous leader certificate
        // Set the leader certificate.
        let leader_certificate = sample_batch_certificate(rng);
        *bft.leader_certificate.write() = Some(leader_certificate);
        // Ensure this call succeeds on an odd round.
        let result = bft.is_leader_quorum_or_nonleaders_available(1);
        assert!(result); // should now fall through to the end of function

        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_is_leader_quorum_even_out_of_sync() -> Result<()> {
        let rng = &mut TestRng::default();

        // Sample the test instance.
        let (committee, account, ledger, storage) = sample_test_instance(Some(1), 10, rng);
        assert_eq!(committee.starting_round(), 1);
        assert_eq!(storage.current_round(), 1);
        assert_eq!(storage.max_gc_rounds(), 10);

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

        // Sample the test instance.
        let (committee, account, ledger, storage) = sample_test_instance(Some(2), 10, rng);
        assert_eq!(committee.starting_round(), 2);
        assert_eq!(storage.current_round(), 2);
        assert_eq!(storage.max_gc_rounds(), 10);

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

        // Sample batch certificates.
        let mut certificates = IndexSet::new();
        certificates.insert(sample_batch_certificate_for_round(2, rng));
        certificates.insert(sample_batch_certificate_for_round(2, rng));
        certificates.insert(sample_batch_certificate_for_round(2, rng));
        certificates.insert(sample_batch_certificate_for_round(2, rng));

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
        let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), 10);
        // Initialize the account.
        let account = Account::new(rng)?;
        // Initialize the BFT.
        let bft = BFT::new(account.clone(), storage.clone(), ledger.clone(), None, &[], None)?;
        // Set the leader certificate.
        let leader_certificate = sample_batch_certificate_for_round(2, rng);
        *bft.leader_certificate.write() = Some(leader_certificate);
        let result = bft.is_even_round_ready_for_next_round(IndexSet::new(), committee.clone(), 2);
        // If leader certificate is set but quorum threshold is not reached, we are not ready for the next round.
        assert!(!result);
        // Once quorum threshold is reached, we are ready for the next round.
        let result = bft.is_even_round_ready_for_next_round(certificates.clone(), committee.clone(), 2);
        assert!(result);

        // Initialize a new BFT.
        let bft_timer = BFT::new(account.clone(), storage.clone(), ledger.clone(), None, &[], None)?;
        // If the leader certificate is not set and the timer has not expired, we are not ready for the next round.
        let result = bft_timer.is_even_round_ready_for_next_round(certificates.clone(), committee.clone(), 2);
        if !bft_timer.is_timer_expired() {
            assert!(!result);
        }
        // Wait for the timer to expire.
        let leader_certificate_timeout =
            std::time::Duration::from_millis(MAX_LEADER_CERTIFICATE_DELAY_IN_SECS as u64 * 1000);
        std::thread::sleep(leader_certificate_timeout);
        // Once the leader certificate timer has expired and quorum threshold is reached, we are ready to advance to the next round.
        let result = bft_timer.is_even_round_ready_for_next_round(certificates.clone(), committee.clone(), 2);
        if bft_timer.is_timer_expired() {
            assert!(result);
        } else {
            assert!(!result);
        }

        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_update_leader_certificate_odd() -> Result<()> {
        let rng = &mut TestRng::default();

        // Sample the test instance.
        let (_, account, ledger, storage) = sample_test_instance(None, 10, rng);
        assert_eq!(storage.max_gc_rounds(), 10);

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

        // Sample the test instance.
        let (_, account, ledger, storage) = sample_test_instance(None, 10, rng);
        assert_eq!(storage.max_gc_rounds(), 10);

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
        let transmissions = Arc::new(BFTMemoryService::new());
        let storage = Storage::new(ledger.clone(), transmissions, 10);
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

        // Sample the test instance.
        let (_, account, ledger, _) = sample_test_instance(Some(1), 10, rng);

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
            let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), 1);
            // Initialize the BFT.
            let bft = BFT::new(account.clone(), storage, ledger.clone(), None, &[], None)?;

            // Insert a mock DAG in the BFT.
            *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(3);

            // Insert the previous certificates into the BFT.
            for certificate in previous_certificates.clone() {
                assert!(bft.update_dag::<false, false>(certificate).await.is_ok());
            }

            // Ensure this call succeeds and returns all given certificates.
            let result = bft.order_dag_with_dfs::<false>(certificate.clone());
            assert!(result.is_ok());
            let candidate_certificates = result.unwrap().into_values().flatten().collect::<Vec<_>>();
            assert_eq!(candidate_certificates.len(), 1);
            let expected_certificates = vec![certificate.clone()];
            assert_eq!(
                candidate_certificates.iter().map(|c| c.id()).collect::<Vec<_>>(),
                expected_certificates.iter().map(|c| c.id()).collect::<Vec<_>>()
            );
            assert_eq!(candidate_certificates, expected_certificates);
        }

        /* Test normal case */

        // Ensure the function succeeds in returning all given certificates.
        {
            // Initialize the storage.
            let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), 1);
            // Initialize the BFT.
            let bft = BFT::new(account, storage, ledger, None, &[], None)?;

            // Insert a mock DAG in the BFT.
            *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(2);

            // Insert the previous certificates into the BFT.
            for certificate in previous_certificates.clone() {
                assert!(bft.update_dag::<false, false>(certificate).await.is_ok());
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
                candidate_certificates.iter().map(|c| c.id()).collect::<Vec<_>>(),
                expected_certificates.iter().map(|c| c.id()).collect::<Vec<_>>()
            );
            assert_eq!(candidate_certificates, expected_certificates);
        }

        Ok(())
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_order_dag_with_dfs_fails_on_missing_previous_certificate() -> Result<()> {
        let rng = &mut TestRng::default();

        // Sample the test instance.
        let (committee, account, ledger, storage) = sample_test_instance(Some(1), 1, rng);
        assert_eq!(committee.starting_round(), 1);
        assert_eq!(storage.current_round(), 1);
        assert_eq!(storage.max_gc_rounds(), 1);

        // Initialize the round parameters.
        let previous_round = 2; // <- This must be an even number, for `BFT::update_dag` to behave correctly below.
        let current_round = previous_round + 1;

        // Sample the current certificate and previous certificates.
        let (certificate, previous_certificates) = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_with_previous_certificates(
            current_round,
            rng,
        );
        // Construct the previous certificate IDs.
        let previous_certificate_ids: IndexSet<_> = previous_certificates.iter().map(|c| c.id()).collect();

        /* Test missing previous certificate. */

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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_bft_gc_on_commit() -> Result<()> {
        let rng = &mut TestRng::default();

        // Initialize the round parameters.
        let max_gc_rounds = 1;
        let committee_round = 0;
        let commit_round = 2;
        let current_round = commit_round + 1;

        // Sample the certificates.
        let (_, certificates) = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_with_previous_certificates(
            current_round,
            rng,
        );

        // Initialize the committee.
        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round_and_members(
            committee_round,
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
        let transmissions = Arc::new(BFTMemoryService::new());
        let storage = Storage::new(ledger.clone(), transmissions, max_gc_rounds);
        // Insert the certificates into the storage.
        for certificate in certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }

        // Get the leader certificate.
        let leader = committee.get_leader(commit_round).unwrap();
        let leader_certificate = storage.get_certificate_for_round_with_author(commit_round, leader).unwrap();

        // Initialize the BFT.
        let account = Account::new(rng)?;
        let bft = BFT::new(account, storage.clone(), ledger, None, &[], None)?;
        // Insert a mock DAG in the BFT.
        *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(commit_round);

        // Ensure that the `gc_round` has not been updated yet.
        assert_eq!(bft.storage().gc_round(), committee_round.saturating_sub(max_gc_rounds));

        // Insert the certificates into the BFT.
        for certificate in certificates {
            assert!(bft.update_dag::<false, false>(certificate).await.is_ok());
        }

        // Commit the leader certificate.
        bft.commit_leader_certificate::<false, false>(leader_certificate).await.unwrap();

        // Ensure that the `gc_round` has been updated.
        assert_eq!(bft.storage().gc_round(), commit_round - max_gc_rounds);

        Ok(())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_sync_bft_dag_at_bootup() -> Result<()> {
        let rng = &mut TestRng::default();

        // Initialize the round parameters.
        let max_gc_rounds = 1;
        let committee_round = 0;
        let commit_round = 2;
        let current_round = commit_round + 1;

        // Sample the current certificate and previous certificates.
        let (_, certificates) = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate_with_previous_certificates(
            current_round,
            rng,
        );

        // Initialize the committee.
        let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round_and_members(
            committee_round,
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
        let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);
        // Insert the certificates into the storage.
        for certificate in certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }

        // Get the leader certificate.
        let leader = committee.get_leader(commit_round).unwrap();
        let leader_certificate = storage.get_certificate_for_round_with_author(commit_round, leader).unwrap();

        // Initialize the BFT.
        let account = Account::new(rng)?;
        let bft = BFT::new(account.clone(), storage, ledger.clone(), None, &[], None)?;

        // Insert a mock DAG in the BFT.
        *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(commit_round);

        // Insert the previous certificates into the BFT.
        for certificate in certificates.clone() {
            assert!(bft.update_dag::<false, false>(certificate).await.is_ok());
        }

        // Commit the leader certificate.
        bft.commit_leader_certificate::<false, false>(leader_certificate.clone()).await.unwrap();

        // Simulate a bootup of the BFT.

        // Initialize a new instance of storage.
        let storage_2 = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);
        // Initialize a new instance of BFT.
        let bootup_bft = BFT::new(account, storage_2, ledger, None, &[], None)?;

        // Sync the BFT DAG at bootup.
        bootup_bft.sync_bft_dag_at_bootup(certificates.clone()).await;

        // Check that the BFT starts from the same last committed round.
        assert_eq!(bft.dag.read().last_committed_round(), bootup_bft.dag.read().last_committed_round());

        // Ensure that both BFTs have committed the leader certificate.
        assert!(bft.dag.read().is_recently_committed(leader_certificate.round(), leader_certificate.id()));
        assert!(bootup_bft.dag.read().is_recently_committed(leader_certificate.round(), leader_certificate.id()));

        // Check the state of the bootup BFT.
        for certificate in certificates {
            let certificate_round = certificate.round();
            let certificate_id = certificate.id();
            // Check that the bootup BFT has committed the certificates.
            assert!(bootup_bft.dag.read().is_recently_committed(certificate_round, certificate_id));
            // Check that the bootup BFT does not contain the certificates in its graph, because
            // it should not need to order them again in subsequent subdags.
            assert!(!bootup_bft.dag.read().contains_certificate_in_round(certificate_round, certificate_id));
        }

        Ok(())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_sync_bft_dag_at_bootup_shutdown() -> Result<()> {
        /*
        1. Run one uninterrupted BFT on a set of certificates for 2 leader commits.
        2. Run a separate bootup BFT that syncs with a set of pre shutdown certificates, and then commits a second leader normally over a set of post shutdown certificates.
        3. Observe that the uninterrupted BFT and the bootup BFT end in the same state.
        */

        let rng = &mut TestRng::default();

        // Initialize the round parameters.
        let max_gc_rounds = snarkvm::ledger::narwhal::BatchHeader::<CurrentNetwork>::MAX_GC_ROUNDS as u64;
        let committee_round = 0;
        let commit_round = 2;
        let current_round = commit_round + 1;
        let next_round = current_round + 1;

        // Sample 5 rounds of batch certificates starting at the genesis round from a static set of 4 authors.
        let (round_to_certificates_map, committee) = {
            let private_keys = vec![
                PrivateKey::new(rng).unwrap(),
                PrivateKey::new(rng).unwrap(),
                PrivateKey::new(rng).unwrap(),
                PrivateKey::new(rng).unwrap(),
            ];
            let addresses = vec![
                Address::try_from(private_keys[0])?,
                Address::try_from(private_keys[1])?,
                Address::try_from(private_keys[2])?,
                Address::try_from(private_keys[3])?,
            ];
            let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round_and_members(
                committee_round,
                addresses,
                rng,
            );
            // Initialize a mapping from the round number to the set of batch certificates in the round.
            let mut round_to_certificates_map: IndexMap<
                u64,
                IndexSet<snarkvm::ledger::narwhal::BatchCertificate<CurrentNetwork>>,
            > = IndexMap::new();
            let mut previous_certificates = IndexSet::with_capacity(4);
            // Initialize the genesis batch certificates.
            for _ in 0..4 {
                previous_certificates.insert(sample_batch_certificate(rng));
            }
            for round in 0..commit_round + 3 {
                let mut current_certificates = IndexSet::new();
                let previous_certificate_ids: IndexSet<_> = if round == 0 || round == 1 {
                    IndexSet::new()
                } else {
                    previous_certificates.iter().map(|c| c.id()).collect()
                };
                let transmission_ids =
                    snarkvm::ledger::narwhal::transmission_id::test_helpers::sample_transmission_ids(rng)
                        .into_iter()
                        .collect::<IndexSet<_>>();
                let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
                let committee_id = committee.id();
                for (i, private_key_1) in private_keys.iter().enumerate() {
                    let batch_header = snarkvm::ledger::narwhal::BatchHeader::new(
                        private_key_1,
                        round,
                        timestamp,
                        committee_id,
                        transmission_ids.clone(),
                        previous_certificate_ids.clone(),
                        rng,
                    )
                    .unwrap();
                    let mut signatures = IndexSet::with_capacity(4);
                    for (j, private_key_2) in private_keys.iter().enumerate() {
                        if i != j {
                            signatures.insert(private_key_2.sign(&[batch_header.batch_id()], rng).unwrap());
                        }
                    }
                    let certificate =
                        snarkvm::ledger::narwhal::BatchCertificate::from(batch_header, signatures).unwrap();
                    current_certificates.insert(certificate);
                }
                // Update the mapping.
                round_to_certificates_map.insert(round, current_certificates.clone());
                previous_certificates = current_certificates.clone();
            }
            (round_to_certificates_map, committee)
        };

        // Initialize the ledger.
        let ledger = Arc::new(MockLedgerService::new(committee.clone()));
        // Initialize the storage.
        let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);
        // Get the leaders for the next 2 commit rounds.
        let leader = committee.get_leader(commit_round).unwrap();
        let next_leader = committee.get_leader(next_round).unwrap();
        // Insert the pre shutdown certificates into the storage.
        let mut pre_shutdown_certificates: Vec<snarkvm::ledger::narwhal::BatchCertificate<CurrentNetwork>> = Vec::new();
        for i in 1..=commit_round {
            let certificates = (*round_to_certificates_map.get(&i).unwrap()).clone();
            if i == commit_round {
                // Only insert the leader certificate for the commit round.
                let leader_certificate = certificates.iter().find(|certificate| certificate.author() == leader);
                if let Some(c) = leader_certificate {
                    pre_shutdown_certificates.push(c.clone());
                }
                continue;
            }
            pre_shutdown_certificates.extend(certificates);
        }
        for certificate in pre_shutdown_certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }
        // Insert the post shutdown certificates into the storage.
        let mut post_shutdown_certificates: Vec<snarkvm::ledger::narwhal::BatchCertificate<CurrentNetwork>> =
            Vec::new();
        for j in commit_round..=commit_round + 2 {
            let certificate = (*round_to_certificates_map.get(&j).unwrap()).clone();
            post_shutdown_certificates.extend(certificate);
        }
        for certificate in post_shutdown_certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }
        // Get the leader certificates.
        let leader_certificate = storage.get_certificate_for_round_with_author(commit_round, leader).unwrap();
        let next_leader_certificate = storage.get_certificate_for_round_with_author(next_round, next_leader).unwrap();

        // Initialize the BFT without bootup.
        let account = Account::new(rng)?;
        let bft = BFT::new(account.clone(), storage, ledger.clone(), None, &[], None)?;

        // Insert a mock DAG in the BFT without bootup.
        *bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(0);

        // Insert the certificates into the BFT without bootup.
        for certificate in pre_shutdown_certificates.clone() {
            assert!(bft.update_dag::<false, false>(certificate).await.is_ok());
        }

        // Insert the post shutdown certificates into the BFT without bootup.
        for certificate in post_shutdown_certificates.clone() {
            assert!(bft.update_dag::<false, false>(certificate).await.is_ok());
        }
        // Commit the second leader certificate.
        let commit_subdag = bft.order_dag_with_dfs::<false>(next_leader_certificate.clone()).unwrap();
        let commit_subdag_metadata = commit_subdag.iter().map(|(round, c)| (*round, c.len())).collect::<Vec<_>>();
        bft.commit_leader_certificate::<false, false>(next_leader_certificate.clone()).await.unwrap();

        // Simulate a bootup of the BFT.

        // Initialize a new instance of storage.
        let bootup_storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);

        // Initialize a new instance of BFT with bootup.
        let bootup_bft = BFT::new(account, bootup_storage.clone(), ledger.clone(), None, &[], None)?;

        // Sync the BFT DAG at bootup.
        bootup_bft.sync_bft_dag_at_bootup(pre_shutdown_certificates.clone()).await;

        // Insert the post shutdown certificates to the storage and BFT with bootup.
        for certificate in post_shutdown_certificates.iter() {
            bootup_bft.storage().testing_only_insert_certificate_testing_only(certificate.clone());
        }
        for certificate in post_shutdown_certificates.clone() {
            assert!(bootup_bft.update_dag::<false, false>(certificate).await.is_ok());
        }
        // Commit the second leader certificate.
        let commit_subdag_bootup = bootup_bft.order_dag_with_dfs::<false>(next_leader_certificate.clone()).unwrap();
        let commit_subdag_metadata_bootup =
            commit_subdag_bootup.iter().map(|(round, c)| (*round, c.len())).collect::<Vec<_>>();
        let committed_certificates_bootup = commit_subdag_bootup.values().flatten();
        bootup_bft.commit_leader_certificate::<false, false>(next_leader_certificate.clone()).await.unwrap();

        // Check that the final state of both BFTs is the same.

        // Check that both BFTs start from the same last committed round.
        assert_eq!(bft.dag.read().last_committed_round(), bootup_bft.dag.read().last_committed_round());

        // Ensure that both BFTs have committed the leader certificates.
        assert!(bft.dag.read().is_recently_committed(leader_certificate.round(), leader_certificate.id()));
        assert!(bft.dag.read().is_recently_committed(next_leader_certificate.round(), next_leader_certificate.id()));
        assert!(bootup_bft.dag.read().is_recently_committed(leader_certificate.round(), leader_certificate.id()));
        assert!(
            bootup_bft.dag.read().is_recently_committed(next_leader_certificate.round(), next_leader_certificate.id())
        );

        // Check that the bootup BFT has committed the pre shutdown certificates.
        for certificate in pre_shutdown_certificates.clone() {
            let certificate_round = certificate.round();
            let certificate_id = certificate.id();
            // Check that both BFTs have committed the certificates.
            assert!(bft.dag.read().is_recently_committed(certificate_round, certificate_id));
            assert!(bootup_bft.dag.read().is_recently_committed(certificate_round, certificate_id));
            // Check that the bootup BFT does not contain the certificates in its graph, because
            // it should not need to order them again in subsequent subdags.
            assert!(!bft.dag.read().contains_certificate_in_round(certificate_round, certificate_id));
            assert!(!bootup_bft.dag.read().contains_certificate_in_round(certificate_round, certificate_id));
        }

        // Check that that the bootup BFT has committed the subdag stemming from the second leader certificate in consensus.
        for certificate in committed_certificates_bootup.clone() {
            let certificate_round = certificate.round();
            let certificate_id = certificate.id();
            // Check that the both BFTs have committed the certificates.
            assert!(bft.dag.read().is_recently_committed(certificate_round, certificate_id));
            assert!(bootup_bft.dag.read().is_recently_committed(certificate_round, certificate_id));
            // Check that the bootup BFT does not contain the certificates in its graph, because
            // it should not need to order them again in subsequent subdags.
            assert!(!bft.dag.read().contains_certificate_in_round(certificate_round, certificate_id));
            assert!(!bootup_bft.dag.read().contains_certificate_in_round(certificate_round, certificate_id));
        }

        // Check that the commit subdag metadata for the second leader is the same for both BFTs.
        assert_eq!(commit_subdag_metadata_bootup, commit_subdag_metadata);

        Ok(())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_sync_bft_dag_at_bootup_dfs() -> Result<()> {
        /*
        1. Run a bootup BFT that syncs with a set of pre shutdown certificates.
        2. Add post shutdown certificates to the bootup BFT.
        2. Observe that in the commit subdag of the second leader certificate, there are no repeated vertices from the pre shutdown certificates.
        */

        let rng = &mut TestRng::default();

        // Initialize the round parameters.
        let max_gc_rounds = snarkvm::ledger::narwhal::BatchHeader::<CurrentNetwork>::MAX_GC_ROUNDS as u64;
        let committee_round = 0;
        let commit_round = 2;
        let current_round = commit_round + 1;
        let next_round = current_round + 1;

        // Sample 5 rounds of batch certificates starting at the genesis round from a static set of 4 authors.
        let (round_to_certificates_map, committee) = {
            let private_keys = vec![
                PrivateKey::new(rng).unwrap(),
                PrivateKey::new(rng).unwrap(),
                PrivateKey::new(rng).unwrap(),
                PrivateKey::new(rng).unwrap(),
            ];
            let addresses = vec![
                Address::try_from(private_keys[0])?,
                Address::try_from(private_keys[1])?,
                Address::try_from(private_keys[2])?,
                Address::try_from(private_keys[3])?,
            ];
            let committee = snarkvm::ledger::committee::test_helpers::sample_committee_for_round_and_members(
                committee_round,
                addresses,
                rng,
            );
            // Initialize a mapping from the round number to the set of batch certificates in the round.
            let mut round_to_certificates_map: IndexMap<
                u64,
                IndexSet<snarkvm::ledger::narwhal::BatchCertificate<CurrentNetwork>>,
            > = IndexMap::new();
            let mut previous_certificates = IndexSet::with_capacity(4);
            // Initialize the genesis batch certificates.
            for _ in 0..4 {
                previous_certificates.insert(sample_batch_certificate(rng));
            }
            for round in 0..=commit_round + 2 {
                let mut current_certificates = IndexSet::new();
                let previous_certificate_ids: IndexSet<_> = if round == 0 || round == 1 {
                    IndexSet::new()
                } else {
                    previous_certificates.iter().map(|c| c.id()).collect()
                };
                let transmission_ids =
                    snarkvm::ledger::narwhal::transmission_id::test_helpers::sample_transmission_ids(rng)
                        .into_iter()
                        .collect::<IndexSet<_>>();
                let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
                let committee_id = committee.id();
                for (i, private_key_1) in private_keys.iter().enumerate() {
                    let batch_header = snarkvm::ledger::narwhal::BatchHeader::new(
                        private_key_1,
                        round,
                        timestamp,
                        committee_id,
                        transmission_ids.clone(),
                        previous_certificate_ids.clone(),
                        rng,
                    )
                    .unwrap();
                    let mut signatures = IndexSet::with_capacity(4);
                    for (j, private_key_2) in private_keys.iter().enumerate() {
                        if i != j {
                            signatures.insert(private_key_2.sign(&[batch_header.batch_id()], rng).unwrap());
                        }
                    }
                    let certificate =
                        snarkvm::ledger::narwhal::BatchCertificate::from(batch_header, signatures).unwrap();
                    current_certificates.insert(certificate);
                }
                // Update the mapping.
                round_to_certificates_map.insert(round, current_certificates.clone());
                previous_certificates = current_certificates.clone();
            }
            (round_to_certificates_map, committee)
        };

        // Initialize the ledger.
        let ledger = Arc::new(MockLedgerService::new(committee.clone()));
        // Initialize the storage.
        let storage = Storage::new(ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);
        // Get the leaders for the next 2 commit rounds.
        let leader = committee.get_leader(commit_round).unwrap();
        let next_leader = committee.get_leader(next_round).unwrap();
        // Insert the pre shutdown certificates into the storage.
        let mut pre_shutdown_certificates: Vec<snarkvm::ledger::narwhal::BatchCertificate<CurrentNetwork>> = Vec::new();
        for i in 1..=commit_round {
            let certificates = (*round_to_certificates_map.get(&i).unwrap()).clone();
            if i == commit_round {
                // Only insert the leader certificate for the commit round.
                let leader_certificate = certificates.iter().find(|certificate| certificate.author() == leader);
                if let Some(c) = leader_certificate {
                    pre_shutdown_certificates.push(c.clone());
                }
                continue;
            }
            pre_shutdown_certificates.extend(certificates);
        }
        for certificate in pre_shutdown_certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }
        // Initialize the bootup BFT.
        let account = Account::new(rng)?;
        let bootup_bft = BFT::new(account.clone(), storage.clone(), ledger.clone(), None, &[], None)?;
        // Insert a mock DAG in the BFT without bootup.
        *bootup_bft.dag.write() = crate::helpers::dag::test_helpers::mock_dag_with_modified_last_committed_round(0);
        // Sync the BFT DAG at bootup.
        bootup_bft.sync_bft_dag_at_bootup(pre_shutdown_certificates.clone()).await;

        // Insert the post shutdown certificates into the storage.
        let mut post_shutdown_certificates: Vec<snarkvm::ledger::narwhal::BatchCertificate<CurrentNetwork>> =
            Vec::new();
        for j in commit_round..=commit_round + 2 {
            let certificate = (*round_to_certificates_map.get(&j).unwrap()).clone();
            post_shutdown_certificates.extend(certificate);
        }
        for certificate in post_shutdown_certificates.iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }

        // Insert the post shutdown certificates into the DAG.
        for certificate in post_shutdown_certificates.clone() {
            assert!(bootup_bft.update_dag::<false, false>(certificate).await.is_ok());
        }

        // Get the next leader certificate to commit.
        let next_leader_certificate = storage.get_certificate_for_round_with_author(next_round, next_leader).unwrap();
        let commit_subdag = bootup_bft.order_dag_with_dfs::<false>(next_leader_certificate).unwrap();
        let committed_certificates = commit_subdag.values().flatten();

        // Check that none of the certificates synced from the bootup appear in the subdag for the next commit round.
        for pre_shutdown_certificate in pre_shutdown_certificates.clone() {
            for committed_certificate in committed_certificates.clone() {
                assert_ne!(pre_shutdown_certificate.id(), committed_certificate.id());
            }
        }
        Ok(())
    }
}
