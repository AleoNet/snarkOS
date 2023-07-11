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
    helpers::{check_timestamp_for_liveness, now, Committee},
    MAX_EXPIRATION_TIME_IN_SECS,
};
use snarkvm::{
    console::{
        account::{Address, Signature},
        network::Network,
        types::Field,
    },
    ledger::narwhal::{Batch, BatchCertificate, Transmission, TransmissionID},
    prelude::{bail, ensure, Result},
};

use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;

pub struct Proposal<N: Network> {
    /// The committee for the round.
    committee: Committee<N>,
    /// The proposed batch.
    batch: Batch<N>,
    /// The map of `(signature, timestamp)` entries.
    signatures: IndexMap<Signature<N>, i64>,
}

impl<N: Network> Proposal<N> {
    /// Initializes a new instance of the proposal.
    pub fn new(committee: Committee<N>, batch: Batch<N>) -> Result<Self> {
        // Ensure the committee round batches the proposed batch round.
        ensure!(committee.round() == batch.round(), "The committee round does not match the batch round");
        // Ensure the batch author is a member of the committee.
        ensure!(committee.is_committee_member(batch.author()), "The batch author is not a committee member");
        // Return the proposal.
        Ok(Self { committee, batch, signatures: Default::default() })
    }

    /// Returns the proposed batch.
    pub const fn batch(&self) -> &Batch<N> {
        &self.batch
    }

    /// Returns the proposed batch ID.
    pub const fn batch_id(&self) -> Field<N> {
        self.batch.batch_id()
    }

    /// Returns the round.
    pub const fn round(&self) -> u64 {
        self.batch.round()
    }

    /// Returns the timestamp.
    pub const fn timestamp(&self) -> i64 {
        self.batch.timestamp()
    }

    /// Returns the transmissions.
    pub const fn transmissions(&self) -> &IndexMap<TransmissionID<N>, Transmission<N>> {
        self.batch.transmissions()
    }

    /// Returns the transmissions.
    pub fn into_transmissions(self) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        self.batch.into_transmissions()
    }

    /// Returns the map of `(signature, timestamp)` entries.
    pub const fn signatures(&self) -> &IndexMap<Signature<N>, i64> {
        &self.signatures
    }

    /// Returns the signers.
    pub fn signers(&self) -> IndexSet<Address<N>> {
        self.signatures.keys().map(Signature::to_address).collect()
    }

    /// Returns the nonsigners.
    pub fn nonsigners(&self) -> IndexSet<Address<N>> {
        // Retrieve the current signers.
        let signers = self.signers();
        // Initialize a set for the non-signers.
        let mut nonsigners = IndexSet::new();
        // Iterate through the committee members.
        for address in self.committee.members().keys() {
            // Insert the address if it is not a signer.
            if !signers.contains(address) {
                nonsigners.insert(*address);
            }
        }
        // Return the non-signers.
        nonsigners
    }

    /// Returns `true` if the proposal has expired.
    pub fn is_expired(&self) -> bool {
        now().saturating_sub(self.timestamp()) > MAX_EXPIRATION_TIME_IN_SECS
    }

    /// Returns `true` if the quorum threshold has been reached for the proposed batch.
    pub fn is_quorum_threshold_reached(&self) -> bool {
        // Construct an iterator over the signers.
        let signers = self.signatures.keys().chain([self.batch.signature()].into_iter()).map(Signature::to_address);
        // Check if the batch has reached the quorum threshold.
        self.committee.is_quorum_threshold_reached(&signers.collect())
    }

    /// Returns `true` if the proposal contains the given transmission ID.
    pub fn contains_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.batch.contains(transmission_id)
    }

    /// Returns the `transmission` for the given `transmission ID`.
    pub fn get_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<&Transmission<N>> {
        self.batch.get(transmission_id)
    }

    /// Adds a signature to the proposal, if the signature is valid.
    pub fn add_signature(&mut self, signer: Address<N>, signature: Signature<N>, timestamp: i64) -> Result<()> {
        // Ensure the signer is in the committee.
        if !self.committee.is_committee_member(signer) {
            bail!("Signature is from a non-committee peer '{signer}'")
        }
        // Ensure the signer is new.
        if self.signers().contains(&signer) {
            bail!("Signature is from a duplicate peer '{signer}'")
        }
        // Verify the signature.
        // Note: This check ensures the peer's address matches the address of the signature.
        if !signature.verify(&signer, &[self.batch_id(), Field::from_u64(timestamp as u64)]) {
            bail!("Signature verification failed")
        }
        // Check the timestamp for liveness.
        check_timestamp_for_liveness(timestamp)?;
        // Insert the signature.
        self.signatures.insert(signature, timestamp);
        Ok(())
    }

    /// Returns the batch certificate and transmissions.
    pub fn to_certificate(&self) -> Result<(BatchCertificate<N>, HashMap<TransmissionID<N>, Transmission<N>>)> {
        // Ensure the quorum threshold has been reached.
        ensure!(self.is_quorum_threshold_reached(), "The quorum threshold has not been reached");
        // Create the batch certificate.
        let certificate = BatchCertificate::new(self.batch.to_header()?, self.signatures.clone())?;
        // Create the transmissions map.
        let transmissions = self.batch.transmissions().clone().into_iter().collect();
        // Return the certificate and transmissions.
        Ok((certificate, transmissions))
    }
}
