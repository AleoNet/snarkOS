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

/// The storage for the memory pool.
///
/// The storage is used to store the following:
/// - `round` to `certificate ID` entries.
/// - `certificate ID` to `certificate` entries.
/// - `batch ID` to `round` entries.
/// - `transmission ID` to `transmission` entries.
///
/// The chain of events is as follows:
/// 1. A `transmission` is received.
/// 2. The `transmission` is added to the `transmissions` map.
/// 3. After a `batch` is ready to be stored:
///   - The `certificate` triggers updates to the `rounds`, `certificates`, and `batch_ids` maps.
#[derive(Clone, Debug)]
pub struct Storage<N: Network> {
    /// The map of `round` to a list of `(certificate ID, batch ID, address)` entries.
    rounds: Arc<RwLock<IndexMap<u64, IndexSet<(Field<N>, Field<N>, Address<N>)>>>>,
    /// The map of `certificate ID` to `certificate`.
    certificates: Arc<RwLock<IndexMap<Field<N>, BatchCertificate<N>>>>,
    /// The map of `batch ID` to `round`.
    batch_ids: Arc<RwLock<IndexMap<Field<N>, u64>>>,
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
        Self {
            rounds: Default::default(),
            certificates: Default::default(),
            batch_ids: Default::default(),
            transmissions: Default::default(),
        }
    }
}

impl<N: Network> Storage<N> {
    /// Returns an iterator over the `rounds` map.
    pub fn rounds_iter(&self) -> impl Iterator<Item = (u64, IndexSet<(Field<N>, Field<N>, Address<N>)>)> {
        self.rounds.read().clone().into_iter()
    }

    /// Returns an iterator over the `certificates` map.
    pub fn certificates_iter(&self) -> impl Iterator<Item = (Field<N>, BatchCertificate<N>)> {
        self.certificates.read().clone().into_iter()
    }

    /// Returns an iterator over the `batch IDs` map.
    pub fn batch_ids_iter(&self) -> impl Iterator<Item = (Field<N>, u64)> {
        self.batch_ids.read().clone().into_iter()
    }

    /// Returns an iterator over the `transmissions` map.
    pub fn transmissions_iter(&self) -> impl Iterator<Item = (TransmissionID<N>, Transmission<N>)> {
        self.transmissions.read().clone().into_iter()
    }
}

impl<N: Network> Storage<N> {
    /// Returns `true` if the storage contains the specified `batch ID`.
    pub fn contains_batch(&self, batch_id: Field<N>) -> bool {
        // Check if the batch ID exists in storage.
        self.batch_ids.read().contains_key(&batch_id)
    }
    
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
            .flat_map(|(certificate_id, _, _)| self.certificates.read().get(certificate_id).cloned())
            .collect();
        // Return the certificates.
        Some(certificates)
    }

    /// Inserts the given `certificate` into storage.
    /// This method triggers updates to the `rounds`, `certificates`, and `batch_ids` maps.
    pub fn insert_certificate(&self, certificate: BatchCertificate<N>) -> Result<()> {
        // Retrieve the round.
        let round = certificate.round();
        // Compute the certificate ID.
        let certificate_id = certificate.to_id()?;
        // Retrieve the batch ID.
        let batch_id = certificate.batch_id();
        // Compute the address of the batch creator.
        let address = certificate.to_address();
        // Ensure the certificate ID does not already exist in storage.
        if self.certificates.read().contains_key(&certificate_id) {
            bail!("Certificate {certificate_id} already exists in storage");
        }

        // TODO (howardwu): Ensure the certificate is well-formed. If not, do not store.
        // TODO (howardwu): Ensure the round is within range. If not, do not store.
        // TODO (howardwu): Ensure the address is in the committee of the specified round. If not, do not store.
        // TODO (howardwu): Ensure I have all of the transmissions. If not, request them before storing.
        // TODO (howardwu): Ensure I have all of the previous certificates. If not, request them before storing.
        // TODO (howardwu): Ensure the previous certificates have reached 2f+1. If not, do not store.

        // Insert the round to certificate ID entry.
        self.rounds.write().entry(round).or_default().insert((certificate_id, batch_id, address));
        // Insert the certificate.
        self.certificates.write().insert(certificate_id, certificate);
        // Insert the batch ID.
        self.batch_ids.write().insert(batch_id, round);
        Ok(())
    }

    /// Removes the given `certificate ID` from storage.
    /// This method triggers updates to the `rounds`, `certificates`, and `batch_ids` maps.
    pub fn remove_certificate(&self, certificate_id: Field<N>) -> Result<()> {
        // Retrieve the certificate.
        let Some(certificate) = self.get_certificate(certificate_id) else {
            bail!("Certificate {certificate_id} does not exist in storage");
        };
        // Retrieve the round.
        let round = certificate.round();
        // Retrieve the batch ID.
        let batch_id = certificate.batch_id();
        // Compute the address of the batch creator.
        let address = certificate.to_address();

        // Remove the round to certificate ID entry.
        self.rounds.write().entry(round).or_default().remove(&(certificate_id, batch_id, address));
        // If the round is empty, remove it.
        if self.rounds.read().get(&round).map_or(false, |entries| entries.is_empty()) {
            self.rounds.write().remove(&round);
        }
        // Remove the certificate.
        self.certificates.write().remove(&certificate_id);
        // Remove the batch ID.
        self.batch_ids.write().remove(&batch_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::prelude::TestRng;

    use indexmap::indexset;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    /// Returns `true` if the storage is empty.
    fn is_empty<N: Network>(storage: &Storage<N>) -> bool {
        storage.rounds.read().is_empty()
            && storage.certificates.read().is_empty()
            && storage.batch_ids.read().is_empty()
            && storage.transmissions.read().is_empty()
    }

    /// Asserts that the storage matches the expected layout.
    fn assert_storage<N: Network>(
        storage: &Storage<N>,
        rounds: Vec<(u64, IndexSet<(Field<N>, Field<N>, Address<N>)>)>,
        certificates: Vec<(Field<N>, BatchCertificate<N>)>,
        batch_ids: Vec<(Field<N>, u64)>,
        transmissions: Vec<(TransmissionID<N>, Transmission<N>)>,
    ) {
        // Ensure the rounds are well-formed.
        assert_eq!(storage.rounds_iter().collect::<Vec<_>>(), rounds);
        // Ensure the certificates are well-formed.
        assert_eq!(storage.certificates_iter().collect::<Vec<_>>(), certificates);
        // Ensure the batch IDs are well-formed.
        assert_eq!(storage.batch_ids_iter().collect::<Vec<_>>(), batch_ids);
        // Ensure the transmissions are well-formed.
        assert_eq!(storage.transmissions_iter().collect::<Vec<_>>(), transmissions);
    }

    #[test]
    fn test_certificate_insert_remove() {
        let rng = &mut TestRng::default();

        // Create a new storage.
        let storage = Storage::<CurrentNetwork>::new();
        // Ensure the storage is empty.
        assert!(is_empty(&storage));

        // Create a new certificate.
        let certificate = snarkvm::ledger::narwhal::batch_certificate::test_helpers::sample_batch_certificate(rng);
        // Compute the certificate ID.
        let certificate_id = certificate.to_id().unwrap();
        // Retrieve the round.
        let round = certificate.round();
        // Retrieve the batch ID.
        let batch_id = certificate.batch_id();
        // Compute the address of the batch creator.
        let address = certificate.to_address();

        // Insert the certificate.
        storage.insert_certificate(certificate.clone()).unwrap();
        // Ensure the storage is not empty.
        assert!(!is_empty(&storage));
        // Ensure the certificate is stored in the correct round.
        assert_eq!(storage.get_round(round), Some(indexset! { certificate.clone() }));

        // Check that the underlying storage representation is correct.
        {
            // Construct the expected layout for 'rounds'.
            let rounds = vec![(round, indexset! { (certificate_id, batch_id, address) })];
            // Construct the expected layout for 'certificates'.
            let certificates = vec![(certificate_id, certificate.clone())];
            // Construct the expected layout for 'batch_ids'.
            let batch_ids = vec![(batch_id, round)];
            // Construct the expected layout for 'transmissions'.
            let transmissions = vec![];
            // Assert the storage is well-formed.
            assert_storage(&storage, rounds, certificates, batch_ids, transmissions);
        }

        // Retrieve the certificate.
        let candidate_certificate = storage.get_certificate(certificate_id).unwrap();
        // Ensure the retrieved certificate is the same as the inserted certificate.
        assert_eq!(certificate, candidate_certificate);

        // Remove the certificate.
        storage.remove_certificate(certificate_id).unwrap();
        // Ensure the storage is empty.
        assert!(is_empty(&storage));
        // Ensure the certificate is no longer stored in the round.
        assert_eq!(storage.get_round(round), None);
    }
}
