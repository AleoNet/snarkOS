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
    helpers::{init_bft_channels, BFTReceiver, Committee, PrimaryReceiver, PrimarySender, Storage, DAG},
    Ledger,
    Primary,
};
use snarkos_account::Account;
use snarkvm::{
    console::account::Address,
    ledger::narwhal::BatchCertificate,
    prelude::{bail, ensure, Field, Network, Result},
};

use crate::helpers::fmt_id;
use indexmap::{IndexMap, IndexSet};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    sync::Arc,
};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct BFT<N: Network> {
    /// The primary.
    primary: Primary<N>,
    /// The DAG.
    dag: Arc<RwLock<DAG<N>>>,
    /// The batch certificate of the leader from the current even round, if one was present.
    leader_certificate: Arc<RwLock<Option<BatchCertificate<N>>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> BFT<N> {
    /// Initializes a new instance of the BFT.
    pub fn new(account: Account<N>, storage: Storage<N>, ledger: Ledger<N>, dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            primary: Primary::new(account, storage, ledger, dev)?,
            dag: Default::default(),
            leader_certificate: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the BFT instance.
    pub async fn run(&mut self, primary_sender: PrimarySender<N>, primary_receiver: PrimaryReceiver<N>) -> Result<()> {
        info!("Starting the BFT instance...");
        // Initialize the BFT channels.
        let (bft_sender, bft_receiver) = init_bft_channels::<N>();
        // Run the primary instance.
        self.primary.run(primary_sender, primary_receiver, Some(bft_sender)).await?;
        // Start the BFT handlers.
        self.start_handlers(bft_receiver);
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
}

impl<N: Network> BFT<N> {
    /// Returns the leader of the current even round, if one was present.
    pub fn leader(&self) -> Option<Address<N>> {
        self.leader_certificate.read().as_ref().map(|certificate| certificate.author())
    }

    /// Returns the certificate of the leader from the current even round, if one was present.
    pub const fn leader_certificate(&self) -> &Arc<RwLock<Option<BatchCertificate<N>>>> {
        &self.leader_certificate
    }

    /// Updates the leader certificate to the current even round.
    ///
    /// This method runs on every even round, by determining the leader of the current even round,
    /// and setting the leader certificate to their certificate in the round, if they were present.
    pub fn update_leader_certificate(&self, round: u64) -> Result<()> {
        // Retrieve the current round.
        let current_round = self.storage().current_round();
        // Ensure the current round matches the given round.
        ensure!(current_round == round, "BFT storage reference is out of sync with the current round");
        // If the current round is odd, throw an error.
        if current_round % 2 != 0 {
            bail!("BFT cannot update the leader certificate in an odd round")
        }

        // Retrieve the certificates for the current round.
        let current_certificates = self.storage().get_certificates_for_round(current_round);
        // If there are no current certificates, set the leader certificate to 'None', and return early.
        if current_certificates.is_empty() {
            // Set the leader certificate to 'None'.
            *self.leader_certificate.write() = None;
            return Ok(());
        }

        // Determine the leader of the current even round, using the committee of the current round.
        let leader = match self.storage().get_committee(current_round) {
            Some(committee) => committee.get_leader()?,
            None => bail!("BFT failed to retrieve the committee for the current round"),
        };
        // Find and set the leader certificate, if the leader was present in the current even round.
        *self.leader_certificate.write() =
            current_certificates.into_iter().find(|certificate| certificate.author() == leader);
        Ok(())
    }

    /// Returns 'true' if any of the following conditions hold:
    ///  - The leader certificate reached quorum threshold `(2f + 1)` (in the previous certificates in the current round),
    ///  - The leader certificate is not included up to availability threshold `(f + 1)` (in the previous certificates of the current round),
    ///  - The leader certificate is 'None'.
    pub fn is_leader_quorum_or_nonleaders_available(&self, round: u64) -> Result<bool> {
        // Retrieve the current round.
        let current_round = self.storage().current_round();
        // Ensure the current round matches the given round.
        ensure!(current_round == round, "BFT storage reference is out of sync with the current round");
        // If the current round is even, throw an error.
        if current_round % 2 != 1 {
            bail!("BFT does not compute stakes for the leader certificate in an even round")
        }

        // Retrieve the leader certificate.
        let Some(leader_certificate) = self.leader_certificate.read().clone() else {
            // If there is no leader certificate for the previous round, return 'true'.
            return Ok(true);
        };
        // Retrieve the leader certificate ID.
        let leader_certificate_id = leader_certificate.certificate_id();
        // Retrieve the certificates for the current round.
        let current_certificates = self.storage().get_certificates_for_round(current_round);
        // Retrieve the committee of the current round.
        let Some(current_committee) = self.storage().get_committee(current_round) else {
            bail!("BFT failed to retrieve the committee for the current round")
        };

        // Compute the stake for the leader certificate.
        let (stake_with_leader, stake_without_leader) =
            self.compute_stake_for_leader_certificate(leader_certificate_id, current_certificates, &current_committee)?;
        // Return 'true' if any of the following conditions hold:
        Ok(stake_with_leader >= current_committee.quorum_threshold()
            || stake_without_leader >= current_committee.availability_threshold())
    }

    /// Computes the amount of stake that has & has not signed for the leader certificate.
    fn compute_stake_for_leader_certificate(
        &self,
        leader_certificate_id: Field<N>,
        current_certificates: IndexSet<BatchCertificate<N>>,
        current_committee: &Committee<N>,
    ) -> Result<(u64, u64)> {
        // If there are no current certificates, return early.
        if current_certificates.is_empty() {
            return Ok((0, 0));
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
        Ok((stake_with_leader, stake_without_leader))
    }
}

impl<N: Network> BFT<N> {
    /// Stores the certificate in the DAG, and attempts to commit one or more anchors.
    fn update_to_next_round(&self, current_round: u64) -> Result<()> {
        // Determine if the BFT is ready to update to the next round.
        let is_ready = match current_round % 2 == 0 {
            true => {
                // Update the leader certificate for the current even round.
                self.update_leader_certificate(current_round)?;
                // Return 'true' if there is a leader certificate set for the current even round.
                self.leader_certificate.read().is_some()
            }
            false => self.is_leader_quorum_or_nonleaders_available(current_round)?,
        };

        if current_round % 2 == 0 {
            if let Some(leader_certificate) = self.leader_certificate.read().as_ref() {
                info!("\n\nRound {current_round} elected a leader - {}\n", leader_certificate.author());
            }
        }

        if is_ready {
            // Update to the next committee in storage.
            self.storage().increment_committee_to_next_round()?;
        }
        Ok(())
    }

    /// Stores the certificate in the DAG, and attempts to commit one or more anchors.
    fn update_dag(&self, certificate: BatchCertificate<N>) -> Result<()> {
        // Retrieve the certificate round.
        let certificate_round = certificate.round();
        // Insert the certificate into the DAG.
        self.dag.write().insert(certificate);

        // Construct the commit round.
        let commit_round = certificate_round.saturating_sub(1);
        // If the commit round is odd, return early.
        if commit_round % 2 != 1 {
            return Ok(());
        }
        // If the commit round is at or below the last committed round, return early.
        if commit_round <= self.dag.read().last_committed_round() {
            return Ok(());
        }

        // Retrieve the committee for the commit round.
        let Some(committee) = self.storage().get_committee(commit_round) else {
            bail!("BFT failed to retrieve the committee for commit round {commit_round}");
        };
        // Compute the leader for the commit round.
        let Ok(leader) = committee.get_leader() else {
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
        if !committee.is_availability_threshold_reached(&authors) {
            // If the leader is not ready to be committed, return early.
            trace!("BFT is not ready to commit {commit_round}");
            return Ok(());
        }

        /* Proceeding to commit the leader. */

        // Order all previous leader certificates since the last committed round.
        let mut leader_certificates = vec![leader_certificate.clone()];
        let mut current_certificate = leader_certificate;
        for round in (self.dag.read().last_committed_round() + 2..=commit_round.saturating_sub(2)).rev().step_by(2) {
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

        // Iterate over the leader certificates to commit.
        for leader_certificate in leader_certificates.into_iter().rev() {
            // Retrieve the leader certificate round.
            let leader_round = leader_certificate.round();
            // Compute the commit subdag.
            let commit_subdag = self.order_dag_with_dfs(leader_certificate);
            // Initialize a map for the deduped transmissions.
            let mut transmissions = IndexMap::new();
            // Start from the oldest leader certificate.
            for certificate in commit_subdag.values().flatten() {
                // Update the DAG.
                self.dag.write().commit(certificate.clone(), self.storage().max_gc_rounds());
                // Retrieve the transmissions.
                for transmission_id in certificate.transmission_ids() {
                    // If the transmission already exists in the map, skip it.
                    if transmissions.contains_key(transmission_id) {
                        continue;
                    }
                    // If the transmission already exists in the ledger, skip it.
                    // TODO (howardwu): Check if the transmission exists in the ledger.
                    // Retrieve the transmission.
                    let Some(transmission) = self.storage().get_transmission(*transmission_id) else {
                        bail!("BFT failed to retrieve transmission {}", fmt_id(transmission_id));
                    };
                    // Add the transmission to the set.
                    transmissions.insert(*transmission_id, transmission);
                }
            }
            // Trigger consensus.
            // TODO (howardwu): Trigger consensus.
            info!(
                "\n\nCommitting a subdag from round {leader_round} with {} transmissions: {:?}\n",
                transmissions.len(),
                commit_subdag.iter().map(|(round, certificates)| (round, certificates.len())).collect::<Vec<_>>()
            );
        }
        Ok(())
    }

    /// Returns the certificates to commit.
    fn order_dag_with_dfs(&self, leader_certificate: BatchCertificate<N>) -> BTreeMap<u64, Vec<BatchCertificate<N>>> {
        // Initialize a map for the certificates to commit.
        let mut commit = BTreeMap::<u64, Vec<_>>::new();
        // Initialize a set for the already ordered certificates.
        let mut already_ordered = HashSet::new();
        // Initialize a buffer for the certificates to order.
        let mut buffer = vec![leader_certificate];
        // Iterate over the certificates to order.
        while let Some(certificate) = buffer.pop() {
            // Insert the certificate into the map.
            commit.entry(certificate.round()).or_default().push(certificate.clone());
            // Iterate over the previous certificate IDs.
            for previous_certificate_id in certificate.previous_certificate_ids() {
                let Some(previous_certificate) = self
                    .dag
                    .read()
                    .get_certificate_for_round_with_id(certificate.round() - 1, *previous_certificate_id)
                else {
                    // It is either ordered or below the GC round.
                    continue;
                };
                // If the previous certificate is already ordered, continue.
                if already_ordered.contains(&previous_certificate.certificate_id()) {
                    continue;
                }
                // If the last committed round is the same as the previous certificate round for this author, continue.
                if self
                    .dag
                    .read()
                    .last_committed_authors()
                    .get(&previous_certificate.author())
                    .map_or(false, |round| *round == previous_certificate.round())
                {
                    // If the previous certificate is already ordered, continue.
                    continue;
                }
                // Insert the previous certificate into the set of already ordered certificates.
                already_ordered.insert(previous_certificate.certificate_id());
                // Insert the previous certificate into the buffer.
                buffer.push(previous_certificate);
            }
        }
        // Ensure we only retain certificates that are above the GC round.
        commit.retain(|round, _| round + self.storage().max_gc_rounds() > self.dag.read().last_committed_round());
        // Return the certificates to commit.
        commit
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
                .filter(|c| traversal.iter().any(|p| c.previous_certificate_ids().contains(&p.certificate_id())))
                .collect();
        }
        Ok(traversal.contains(&previous_certificate))
    }
}

impl<N: Network> BFT<N> {
    /// Starts the BFT handlers.
    fn start_handlers(&self, bft_receiver: BFTReceiver<N>) {
        let BFTReceiver { mut rx_primary_round, mut rx_primary_certificate } = bft_receiver;

        // Process the current round from the primary.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((current_round, callback_sender)) = rx_primary_round.recv().await {
                callback_sender.send(self_.update_to_next_round(current_round)).ok();
            }
        });

        // Process the certificate from the primary.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((certificate, callback_sender)) = rx_primary_certificate.recv().await {
                callback_sender.send(Ok(())).ok();
                if let Err(e) = self_.update_dag(certificate) {
                    warn!("BFT failed to update the DAG: {e}");
                }
            }
        });
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the BFT.
    pub async fn shut_down(&self) {
        trace!("Shutting down the BFT...");
        // Shut down the primary.
        self.primary.shut_down().await;
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}
