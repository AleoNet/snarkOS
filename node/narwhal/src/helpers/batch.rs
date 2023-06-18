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

use crate::helpers::{Transmission, TransmissionID};
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{Address, Field, PrivateKey, Signature},
};

use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;

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
    /// The batch ID, defined as the hash of the round number, transmission IDs, and previous batch certificates.
    batch_id: Field<N>,
    /// The round number.
    round: u64,
    /// The timestamp.
    timestamp: i64,
    /// The map of `transmission IDs` to `transmissions`.
    transmissions: HashMap<TransmissionID<N>, Data<Transmission<N>>>,
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
        transmissions: HashMap<TransmissionID<N>, Data<Transmission<N>>>,
        previous_certificates: Vec<BatchCertificate<N>>,
        rng: &mut R,
    ) -> Result<Self> {
        // If the round is zero, then there should be no previous certificates.
        ensure!(round != 0 || previous_certificates.is_empty(), "Invalid round number");
        // If the round is not zero, then there should be at least one previous certificate.
        ensure!(round == 0 || !previous_certificates.is_empty(), "Invalid round number");
        // Checkpoint the timestamp for the batch.
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        // Compute the batch ID.
        let batch_id = Self::compute_batch_id(round, timestamp, &transmissions, &previous_certificates)?;
        // Sign the preimage.
        let signature = private_key.sign(&[batch_id], rng)?;
        // Return the batch.
        Ok(Self { batch_id, round, timestamp, transmissions, previous_certificates, signature })
    }

    /// Returns the batch ID.
    pub const fn batch_id(&self) -> Field<N> {
        self.batch_id
    }

    /// Returns the round number.
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Returns the timestamp.
    pub const fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Returns the transmissions.
    pub const fn transmissions(&self) -> &HashMap<TransmissionID<N>, Data<Transmission<N>>> {
        &self.transmissions
    }

    /// Returns the batch certificates for the previous round.
    pub const fn previous_certificates(&self) -> &Vec<BatchCertificate<N>> {
        &self.previous_certificates
    }

    /// Returns the signature.
    pub const fn signature(&self) -> &Signature<N> {
        &self.signature
    }

    /// Returns the number of transmissions in the batch.
    pub fn len(&self) -> usize {
        self.transmissions.len()
    }

    /// Returns the transmission IDs.
    pub fn transmission_ids(&self) -> Vec<TransmissionID<N>> {
        self.transmissions.keys().copied().collect()
    }

    /// Returns `true` if the batch contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmissions.contains_key(&transmission_id.into())
    }

    /// Returns the transmission, given the specified `transmission ID`.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<&Data<Transmission<N>>> {
        self.transmissions.get(&transmission_id.into())
    }
}

impl<N: Network> Batch<N> {
    /// Returns the batch ID.
    pub fn compute_batch_id(
        round: u64,
        timestamp: i64,
        transmissions: &HashMap<TransmissionID<N>, Data<Transmission<N>>>,
        previous_certificates: &[BatchCertificate<N>],
    ) -> Result<Field<N>> {
        let mut preimage = Vec::new();
        // Insert the round number.
        preimage.extend_from_slice(&round.to_bytes_le()?);
        // Insert the timestamp.
        preimage.extend_from_slice(&timestamp.to_bytes_le()?);
        // Insert the number of transmissions.
        preimage.extend_from_slice(&u64::try_from(transmissions.len())?.to_bytes_le()?);
        // Insert the transmission IDs.
        for transmission_id in transmissions.keys() {
            preimage.extend_from_slice(&transmission_id.to_bytes_le()?);
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
