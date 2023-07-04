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

use snarkvm::{
    ledger::narwhal::{BatchCertificate, Transmission, TransmissionID},
    prelude::{Address, Field, Network},
};

use anyhow::{bail, Result};
use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Storage<N: Network> {
    /// The map of `round` to a list of `(certificate ID, address)` entries.
    rounds: Arc<RwLock<IndexMap<u64, IndexSet<(Field<N>, Address<N>)>>>>,
    /// The map of `certificate ID` to `certificate`.
    certificates: Arc<RwLock<IndexMap<Field<N>, BatchCertificate<N>>>>,
    /// The map of `transmission ID` to `transmission`.
    transmissions: Arc<RwLock<IndexMap<TransmissionID<N>, Transmission<N>>>>,
}

impl<N: Network> Default for Storage<N> {
    /// Initializes a new instance of storage.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Storage<N> {
    /// Initializes a new instance of storage.
    pub fn new() -> Self {
        Self { rounds: Default::default(), certificates: Default::default(), transmissions: Default::default() }
    }
}

impl<N: Network> Storage<N> {
    /// Returns `true` if the storage contains the specified `certificate ID`.
    pub fn contains_certificate(&self, certificate_id: Field<N>) -> bool {
        // Check if the certificate ID exists in storage.
        self.certificates.read().contains_key(&certificate_id)
    }

    /// Returns the certificate for the given `certificate ID`.
    /// If the certificate ID does not exist in storage, `None` is returned.
    pub fn get_certificate(&self, certificate_id: Field<N>) -> Option<BatchCertificate<N>> {
        // Get the batch certificate.
        self.certificates.read().get(&certificate_id).cloned()
    }

    /// Returns the certificates for the given `round`.
    /// If the round does not exist in storage, `None` is returned.
    pub fn get_round(&self, round: u64) -> Option<IndexSet<BatchCertificate<N>>> {
        // The genesis round does not have batch certificates.
        if round == 0 {
            return None;
        }
        // Retrieve the round.
        let Some(entries) = self.rounds.read().get(&round).cloned() else {
            return None;
        };
        // Retrieve the certificates.
        let certificates = entries
            .iter()
            .flat_map(|(certificate_id, _)| self.certificates.read().get(certificate_id).cloned())
            .collect();
        // Return the certificates.
        Some(certificates)
    }

    /// Inserts the given `round` to (`certificate ID`, `certificate`) entry into storage.
    pub fn insert_certificate(&self, certificate: BatchCertificate<N>) -> Result<()> {
        // Retrieve the round.
        let round = certificate.round();
        // Compute the certificate ID.
        let certificate_id = certificate.to_id()?;
        // Compute the address of the batch creator.
        let address = certificate.to_address();
        // Ensure the certificate ID does not already exist in storage.
        if !self.certificates.read().contains_key(&certificate_id) {
            bail!("Certificate {certificate_id} already exists in storage");
        }

        // TODO (howardwu): Ensure the certificate is well-formed. If not, do not store.
        // TODO (howardwu): Ensure the round is within range. If not, do not store.
        // TODO (howardwu): Ensure the address is in the committee of the specified round. If not, do not store.
        // TODO (howardwu): Ensure I have all of the transmissions. If not, request them before storing.
        // TODO (howardwu): Ensure I have all of the previous certificates. If not, request them before storing.
        // TODO (howardwu): Ensure the previous certificates have reached 2f+1. If not, do not store.

        // Insert the round to certificate ID entry.
        self.rounds.write().entry(round).or_default().insert((certificate_id, address));
        // Insert the certificate.
        self.certificates.write().insert(certificate_id, certificate);
        Ok(())
    }
}

impl<N: Network> Storage<N> {
    /// Returns `true` if the storage contains the specified `transmission ID`.
    pub fn contains_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        // Check if the transmission ID exists in storage.
        self.transmissions.read().contains_key(&transmission_id.into())
    }

    /// Returns the transmission for the given `transmission ID`.
    /// If the transmission ID does not exist in storage, `None` is returned.
    pub fn get_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Transmission<N>> {
        // Get the transmission.
        self.transmissions.read().get(&transmission_id.into()).cloned()
    }

    /// Inserts the given (`transmission ID`, `transmission`) into storage.
    /// If the transmission ID already exists in storage, the existing transmission is returned.
    pub fn insert_transmission(
        &self,
        transmission_id: impl Into<TransmissionID<N>>,
        transmission: Transmission<N>,
    ) -> Option<Transmission<N>> {
        // Insert the transmission.
        self.transmissions.write().insert(transmission_id.into(), transmission)
    }
}
