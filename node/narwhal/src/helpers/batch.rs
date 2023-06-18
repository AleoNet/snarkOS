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

use crate::helpers::{Entry, EntryID, Ready};
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{Address, Field, PrivateKey, Signature},
};

use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct SealedBatch<N: Network> {
    /// The batch.
    batch: Batch<N>,
    /// The batch certificate.
    certificate: BatchCertificate<N>,
}

impl<N: Network> SealedBatch<N> {
    /// Initializes a new sealed batch.
    pub fn new(batch: Batch<N>, certificate: BatchCertificate<N>) -> Self {
        Self { batch, certificate }
    }

    /// Returns the batch.
    pub const fn batch(&self) -> &Batch<N> {
        &self.batch
    }

    /// Returns the batch certificate.
    pub const fn certificate(&self) -> &BatchCertificate<N> {
        &self.certificate
    }
}

#[derive(Clone, Debug)]
pub struct BatchCertificate<N: Network> {
    /// The batch ID.
    batch_id: Field<N>,
    /// The `signatures` of the batch ID from the committee.
    signatures: Vec<Signature<N>>,
}

#[derive(Clone, Debug)]
pub struct Batch<N: Network> {
    /// The batch ID, defined as the hash of the round number, entry IDs, and previous batch certificates.
    batch_id: Field<N>,
    /// The round number.
    round: u64,
    /// The map of `entry IDs` to `entries`.
    entries: HashMap<EntryID<N>, Data<Entry<N>>>,
    /// The batch certificates of the previous round.
    previous_certificates: Vec<BatchCertificate<N>>,
    /// The signature of the batch ID from the creator.
    signature: Signature<N>,
}

impl<N: Network> Batch<N> {
    /// Initializes a new batch.
    pub fn new<R: Rng + CryptoRng>(
        private_key: &PrivateKey<N>,
        round: u64,
        entries: HashMap<EntryID<N>, Data<Entry<N>>>,
        previous_certificates: Vec<BatchCertificate<N>>,
        rng: &mut R,
    ) -> Result<Self> {
        // If the round is zero, then there should be no previous certificates.
        ensure!(round != 0 || previous_certificates.is_empty(), "Invalid round number");
        // If the round is not zero, then there should be at least one previous certificate.
        ensure!(round == 0 || !previous_certificates.is_empty(), "Invalid round number");
        // Compute the batch ID.
        let batch_id = Self::compute_batch_id(round, &entries, &previous_certificates)?;
        // Sign the preimage.
        let signature = private_key.sign(&[batch_id], rng)?;
        // Return the batch.
        Ok(Self { batch_id, round, entries, previous_certificates, signature })
    }

    /// Returns the batch ID.
    pub const fn batch_id(&self) -> Field<N> {
        self.batch_id
    }

    /// Returns the round number.
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Returns the entries.
    pub const fn entries(&self) -> &HashMap<EntryID<N>, Data<Entry<N>>> {
        &self.entries
    }

    /// Returns the batch certificates for the previous round.
    pub const fn previous_certificates(&self) -> &Vec<BatchCertificate<N>> {
        &self.previous_certificates
    }

    /// Returns the number of entries in the batch.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns the entry IDs.
    pub fn entry_ids(&self) -> Vec<EntryID<N>> {
        self.entries.keys().copied().collect()
    }

    /// Returns `true` if the batch contains the specified `entry ID`.
    pub fn contains(&self, entry_id: impl Into<EntryID<N>>) -> bool {
        self.entries.contains_key(&entry_id.into())
    }

    /// Returns the entry, given the specified `entry ID`.
    pub fn get(&self, entry_id: impl Into<EntryID<N>>) -> Option<&Data<Entry<N>>> {
        self.entries.get(&entry_id.into())
    }
}

impl<N: Network> Batch<N> {
    /// Returns the batch ID.
    pub fn compute_batch_id(
        round: u64,
        entries: &HashMap<EntryID<N>, Data<Entry<N>>>,
        previous_certificates: &[BatchCertificate<N>],
    ) -> Result<Field<N>> {
        let mut preimage = Vec::new();
        // Insert the round number.
        preimage.extend_from_slice(&round.to_bytes_le()?);
        // Insert the number of entries.
        preimage.extend_from_slice(&u64::try_from(entries.len())?.to_bytes_le()?);
        // Insert the entry IDs.
        for entry_id in entries.keys() {
            preimage.extend_from_slice(&entry_id.to_bytes_le()?);
        }
        // Insert the number of previous certificates.
        preimage.extend_from_slice(&u64::try_from(previous_certificates.len())?.to_bytes_le()?);
        // Insert the previous certificates.
        for certificate in previous_certificates {
            // Insert the batch ID.
            preimage.extend_from_slice(&certificate.batch_id.to_bytes_le()?);
            // Insert the number of signatures.
            preimage.extend_from_slice(&u64::try_from(certificate.signatures.len())?.to_bytes_le()?);
            // Insert the signatures.
            for signature in &certificate.signatures {
                preimage.extend_from_slice(&signature.to_bytes_le()?);
            }
        }
        // Hash the preimage.
        N::hash_bhp1024(&preimage.to_bits_le())
    }
}
