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
    helpers::{init_bft_channels, BFTReceiver, Committee, PrimaryReceiver, PrimarySender, Storage},
    Primary,
};
use snarkos_account::Account;
use snarkvm::{
    console::account::Address,
    ledger::narwhal::BatchCertificate,
    prelude::{bail, ensure, Field, Network, Result},
};

use indexmap::IndexSet;
use parking_lot::{Mutex, RwLock};
use std::{future::Future, sync::Arc};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct BFT<N: Network> {
    /// The primary.
    primary: Primary<N>,
    /// The batch certificate of the leader from the previous round, if one was present.
    leader_certificate: Arc<RwLock<Option<BatchCertificate<N>>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> BFT<N> {
    /// Initializes a new instance of the BFT.
    pub fn new(storage: Storage<N>, account: Account<N>, dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            primary: Primary::new(storage, account, dev)?,
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
    /// Returns the leader of the previous round, if one was present.
    pub fn leader(&self) -> Option<Address<N>> {
        self.leader_certificate.read().as_ref().map(|certificate| certificate.author())
    }

    /// Returns the certificate of the leader from the previous round, if one was present.
    pub const fn leader_certificate(&self) -> &Arc<RwLock<Option<BatchCertificate<N>>>> {
        &self.leader_certificate
    }

    /// Updates the leader certificate to the previous round.
    ///
    /// This method runs on every even round, by determining the leader of the previous round,
    /// and setting the leader certificate to their certificate in the previous round, if they were present.
    pub fn update_leader_certificate(&self, round: u64) -> Result<()> {
        // Retrieve the current round.
        let current_round = self.storage().current_round();
        // Ensure the current round matches the given round.
        ensure!(current_round == round, "BFT storage reference is out of sync with the current round");
        // If the current round is odd, throw an error.
        if current_round % 2 != 0 {
            bail!("BFT cannot update the leader certificate on an odd round")
        }

        // Retrieve the previous round.
        let previous_round = current_round.saturating_sub(1);
        // Retrieve the certificates for the previous round.
        let previous_certificates = self.storage().get_certificates_for_round(previous_round);
        // If there are no previous certificates, set the previous leader certificate to 'None', and return early.
        if previous_certificates.is_empty() {
            // Set the previous leader certificate to 'None'.
            *self.leader_certificate.write() = None;
            return Ok(());
        }

        // TODO (howardwu): Determine whether to use the current round or the previous round committee.
        // Determine the leader of the previous round, using the committee of the current round.
        let leader = match self.storage().get_committee(current_round) {
            Some(committee) => committee.get_leader()?,
            None => bail!("BFT failed to retrieve the committee for the current round"),
        };
        // Find and set the leader certificate to the leader of the previous round, if they were present.
        *self.leader_certificate.write() =
            previous_certificates.into_iter().find(|certificate| certificate.author() == leader);
        Ok(())
    }

    /// Returns 'true' if any of the following conditions hold:
    ///  - The leader certificate reached quorum threshold `(2f + 1)` (in the previous certificates in the current round),
    ///  - The leader certificate is not included up to availability threshold `(f + 1)` (in the previous certificates of the current round),
    ///  - The leader certificate is 'None'.
    pub fn process_odd_round(&self, round: u64) -> Result<bool> {
        // Retrieve the current round.
        let current_round = self.storage().current_round();
        // Ensure the current round matches the given round.
        ensure!(current_round == round, "BFT storage reference is out of sync with the current round");
        // If the current round is even, throw an error.
        if current_round % 2 != 1 {
            bail!("BFT cannot compute the stake for the leader certificate in an even round")
        }

        // Retrieve the leader certificate.
        let Some(leader_certificate) = self.leader_certificate.read().clone() else {
            // If there is no leader certificate for the previous round, return 'true'.
            return Ok(true)
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
    fn update_to_next_round(&self, round: u64) -> Result<()> {
        let _is_ready = match round % 2 == 0 {
            true => {
                // Update the leader certificate to the previous round.
                self.update_leader_certificate(round)?;
                // Return 'true' if there is a leader certificate set for the previous round.
                self.leader_certificate.read().is_some()
            }
            false => self.process_odd_round(round)?,
        };
        Ok(())
    }
}

impl<N: Network> BFT<N> {
    /// Starts the BFT handlers.
    fn start_handlers(&self, bft_receiver: BFTReceiver<N>) {
        let BFTReceiver { mut rx_primary_round, .. } = bft_receiver;

        // Process the certificate from the primary.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some(round) = rx_primary_round.recv().await {
                if let Err(e) = self_.update_to_next_round(round) {
                    warn!("Cannot process certificate from primary - {e}");
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
