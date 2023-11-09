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

use serde::{Deserialize, Serialize, Serializer, Deserializer};
use snarkvm::{
    console::{
        account::{Address, Signature},
        network::Network,
        types::Field,
    },
    ledger::{
        committee::Committee,
        narwhal::{BatchCertificate, BatchHeader, Transmission, TransmissionID},
    },
    prelude::{bail, ensure, error, Itertools, Result}, utilities::{ToBytesSerializer, FromBytesDeserializer, DeserializeExt, FromBytes, ToBytes},
};
use snarkvm::prelude::SerializeStruct;

use indexmap::{IndexMap, IndexSet};
use std::{collections::HashSet, io::{Write, Read, Result as IoResult}, fmt::{Display, Formatter}, str::FromStr};
use std::io::Error;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Proposal<N: Network> {
    /// The proposed batch header.
    batch_header: BatchHeader<N>,
    /// The proposed transmissions.
    transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    /// The set of signatures.
    signatures: IndexSet<Signature<N>>,
}

impl<N: Network> Proposal<N> {
    /// Initializes a new instance of the proposal.
    pub fn new(
        committee: Committee<N>,
        batch_header: BatchHeader<N>,
        transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<Self> {
        // Ensure the committee is for the batch round.
        ensure!(batch_header.round() >= committee.starting_round(), "Batch round must be >= the committee round");
        // Ensure the batch author is a member of the committee.
        ensure!(committee.is_committee_member(batch_header.author()), "The batch author is not a committee member");
        // Ensure the transmissions are not empty.
        ensure!(!transmissions.is_empty(), "The transmissions are empty");
        // Ensure the transmission IDs match in the batch header and transmissions.
        ensure!(
            batch_header.transmission_ids().len() == transmissions.len(),
            "The transmission IDs do not match in the batch header and transmissions"
        );
        for (a, b) in batch_header.transmission_ids().iter().zip_eq(transmissions.keys()) {
            ensure!(a == b, "The transmission IDs do not match in the batch header and transmissions");
        }
        // Return the proposal.
        Ok(Self { batch_header, transmissions, signatures: Default::default() })
    }

    /// Returns the proposed batch header.
    pub const fn batch_header(&self) -> &BatchHeader<N> {
        &self.batch_header
    }

    /// Returns the proposed batch ID.
    pub const fn batch_id(&self) -> Field<N> {
        self.batch_header.batch_id()
    }

    /// Returns the round.
    pub const fn round(&self) -> u64 {
        self.batch_header.round()
    }

    /// Returns the timestamp.
    pub const fn timestamp(&self) -> i64 {
        self.batch_header.timestamp()
    }

    /// Returns the transmissions.
    pub const fn transmissions(&self) -> &IndexMap<TransmissionID<N>, Transmission<N>> {
        &self.transmissions
    }

    /// Returns the transmissions.
    pub fn into_transmissions(self) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        self.transmissions
    }

    /// Returns the signers.
    pub fn signers(&self) -> HashSet<Address<N>> {
        self.signatures.iter().chain(Some(self.batch_header.signature())).map(Signature::to_address).collect()
    }

    /// Returns the nonsigners.
    pub fn nonsigners(&self, committee: &Committee<N>) -> HashSet<Address<N>> {
        // Retrieve the current signers.
        let signers = self.signers();
        // Initialize a set for the non-signers.
        let mut nonsigners = HashSet::new();
        // Iterate through the committee members.
        for address in committee.members().keys() {
            // Insert the address if it is not a signer.
            if !signers.contains(address) {
                nonsigners.insert(*address);
            }
        }
        // Return the non-signers.
        nonsigners
    }

    /// Returns `true` if the quorum threshold has been reached for the proposed batch.
    pub fn is_quorum_threshold_reached(&self, committee: &Committee<N>) -> bool {
        // Check if the batch has reached the quorum threshold.
        committee.is_quorum_threshold_reached(&self.signers())
    }

    /// Returns `true` if the proposal contains the given transmission ID.
    pub fn contains_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmissions.contains_key(&transmission_id.into())
    }

    /// Returns the `transmission` for the given `transmission ID`.
    pub fn get_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<&Transmission<N>> {
        self.transmissions.get(&transmission_id.into())
    }

    /// Adds a signature to the proposal, if the signature is valid.
    pub fn add_signature(
        &mut self,
        signer: Address<N>,
        signature: Signature<N>,
        committee: &Committee<N>,
    ) -> Result<()> {
        // Ensure the signer is in the committee.
        if !committee.is_committee_member(signer) {
            bail!("Signature from a non-committee member - '{signer}'")
        }
        // Ensure the signer is new.
        if self.signers().contains(&signer) {
            bail!("Duplicate signature from '{signer}'")
        }
        // Verify the signature. If the signature is not valid, return an error.
        // Note: This check ensures the peer's address matches the address of the signature.
        if !signature.verify(&signer, &[self.batch_id()]) {
            bail!("Signature verification failed")
        }
        // Insert the signature.
        self.signatures.insert(signature);
        Ok(())
    }

    /// Returns the batch certificate and transmissions.
    pub fn to_certificate(
        &self,
        committee: &Committee<N>,
    ) -> Result<(BatchCertificate<N>, IndexMap<TransmissionID<N>, Transmission<N>>)> {
        // Ensure the quorum threshold has been reached.
        ensure!(self.is_quorum_threshold_reached(committee), "The quorum threshold has not been reached");
        // Create the batch certificate.
        let certificate = BatchCertificate::from(self.batch_header.clone(), self.signatures.clone())?;
        // Return the certificate and transmissions.
        Ok((certificate, self.transmissions.clone()))
    }
}

impl<N: Network> Serialize for Proposal<N> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut proposal = serializer.serialize_struct("Proposal", 3)?;
                proposal.serialize_field("batch_header", &self.batch_header)?;
                proposal.serialize_field("transmissions", &self.transmissions)?;
                proposal.serialize_field("signatures", &self.signatures)?;
                proposal.end()
            }
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for Proposal<N> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let mut proposal = serde_json::Value::deserialize(deserializer)?;
                Ok(Proposal{
                    batch_header: DeserializeExt::take_from_value::<D>(&mut proposal, "batch_header")?,
                    transmissions: DeserializeExt::take_from_value::<D>(&mut proposal, "transmissions")?,
                    signatures: DeserializeExt::take_from_value::<D>(&mut proposal, "signatures")?,
                })
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "proposal"),
        }
    }
}

impl<N: Network> FromBytes for Proposal<N> {
    /// Reads the transmission from the buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u8::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 1 {
            return Err(error("Invalid proposal version"));
        }
        // Read the batch_header
        let batch_header = BatchHeader::<N>::read_le(&mut reader)?;
        // Read the transmissions
        let num_transmissions = u64::read_le(&mut reader)?;
        let mut transmissions = IndexMap::new();
        for _ in 0..num_transmissions {
            let transmission = <(TransmissionID::<N>, Transmission::<N>)>::read_le(&mut reader)?;
            transmissions.insert(transmission.0, transmission.1);
        }
        // Read the signatures
        let num_signatures = u64::read_le(&mut reader)?;
        let mut signatures = IndexSet::new();
        for _ in 0..num_signatures {
            signatures.insert(Signature::<N>::read_le(&mut reader)?);
        }

        Ok(Self { batch_header, transmissions, signatures })
    }
}

impl<N: Network> ToBytes for Proposal<N> {
    /// Writes the transmission to the buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        1u8.write_le(&mut writer)?;
        // Write the batch_header.
        self.batch_header.write_le(&mut writer)?;
        // Write the transmissions.
        (self.transmissions.len() as u64).write_le(&mut writer)?;
        for t in &self.transmissions {
            t.write_le(&mut writer)?;
        }
        // Write the signatures.
        (self.signatures.len() as u64).write_le(&mut writer)?;
        for s in &self.signatures {
            s.write_le(&mut writer)?;
        }
        Ok(())
    }
}

impl<N: Network> Display for Proposal<N> {
    /// Displays the proposal as a JSON-string.
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).map_err::<std::fmt::Error, _>(serde::ser::Error::custom)?)
    }
}

impl<N: Network> FromStr for Proposal<N> {
    type Err = Error;

    /// Initializes the proposal from a JSON-string.
    fn from_str(header: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(header)?)
    }
}

#[cfg(test)]
mod tests {
    use snarkvm::{utilities::TestRng, prelude::{Testnet3, narwhal::{batch_header::test_helpers::sample_batch_header, transmission::test_helpers::sample_transmissions}}};
    type CurrentNetwork = Testnet3;
    use std::{fmt::{Debug, Display}, str::FromStr};
    use snarkvm::utilities::ToBytes;
    use rand::Rng;

    use super::*;

    /// Returns a sample proposal, sampled at random.
    pub fn sample_proposals(rng: &mut TestRng) -> Vec<Proposal<CurrentNetwork>> {
        let mut sample = Vec::with_capacity(10);
        for _ in 0..10 {
            // let transmission_ids = sample_transmission_ids(rng);
            let transmissions = sample_transmissions(rng);
            let batch_header = sample_batch_header(rng);
            let mut committee = IndexMap::new();
            committee.insert(batch_header.author(), (1000000000000u64, true));
            committee.insert(Address::<CurrentNetwork>::new(rng.gen()), (1000000000000u64, true));
            committee.insert(Address::<CurrentNetwork>::new(rng.gen()), (1000000000000u64, true));
            committee.insert(Address::<CurrentNetwork>::new(rng.gen()), (1000000000000u64, true));
            let transmissions: IndexMap<_,_> = batch_header.transmission_ids().iter().copied().zip_eq(transmissions).collect();
            let proposal = Proposal::new(
                Committee::<CurrentNetwork>::new(1, committee).unwrap(),
                batch_header,
                transmissions,
            ).unwrap();
            sample.push(proposal);
        }
        sample
    }

    #[test]
    fn test_bytes() {
        let rng = &mut TestRng::default();

        for expected in sample_proposals(rng) {
            // Check the byte representation.
            let expected_bytes = expected.to_bytes_le().unwrap();
            assert_eq!(expected, Proposal::read_le(&expected_bytes[..]).unwrap());
        }
    }

    fn check_serde_json<
        T: Serialize + for<'a> Deserialize<'a> + Debug + Display + PartialEq + Eq + FromStr + ToBytes + FromBytes,
    >(
        expected: T,
    ) {
        // Serialize
        let expected_string = &expected.to_string();
        let candidate_string = serde_json::to_string(&expected).unwrap();
        assert_eq!(expected_string, &serde_json::Value::from_str(&candidate_string).unwrap().to_string());

        // Deserialize
        assert_eq!(expected, T::from_str(expected_string).unwrap_or_else(|_| panic!("FromStr: {expected_string}")));
        assert_eq!(expected, serde_json::from_str(&candidate_string).unwrap());
    }

    fn check_bincode<T: Serialize + for<'a> Deserialize<'a> + Debug + PartialEq + Eq + ToBytes + FromBytes>(
        expected: T,
    ) {
        // Serialize
        let expected_bytes = expected.to_bytes_le().unwrap();
        let expected_bytes_with_size_encoding = bincode::serialize(&expected).unwrap();
        assert_eq!(&expected_bytes[..], &expected_bytes_with_size_encoding[8..]);

        // Deserialize
        assert_eq!(expected, T::read_le(&expected_bytes[..]).unwrap());
        assert_eq!(expected, bincode::deserialize(&expected_bytes_with_size_encoding[..]).unwrap());
    }

    #[test]
    fn test_serde_json() {
        let rng = &mut TestRng::default();

        for expected in sample_proposals(rng) {
            check_serde_json(expected);
        }
    }

    #[test]
    fn test_bincode() {
        let rng = &mut TestRng::default();

        for expected in sample_proposals(rng) {
            check_bincode(expected);
        }
    }
}


#[cfg(test)]
mod prop_tests {
    use crate::helpers::{
        now,
        storage::prop_tests::{AnyTransmission, AnyTransmissionID, CryptoTestRng},
        Proposal,
    };
    use snarkvm::ledger::{
        committee::prop_tests::{CommitteeContext, ValidatorSet},
        narwhal::BatchHeader,
    };

    use indexmap::IndexMap;
    use proptest::sample::{size_range, Selector};
    use test_strategy::proptest;

    #[proptest]
    fn initialize_proposal(
        context: CommitteeContext,
        #[any(size_range(1..16).lift())] transmissions: Vec<(AnyTransmissionID, AnyTransmission)>,
        selector: Selector,
        mut rng: CryptoTestRng,
    ) {
        let CommitteeContext(committee, ValidatorSet(validators)) = context;

        let signer = selector.select(&validators);
        let mut transmission_map = IndexMap::new();

        for (AnyTransmissionID(id), AnyTransmission(t)) in transmissions.iter() {
            transmission_map.insert(*id, t.clone());
        }

        let header = BatchHeader::new(
            &signer.private_key,
            committee.starting_round(),
            now(),
            transmission_map.keys().cloned().collect(),
            Default::default(),
            &mut rng,
        )
        .unwrap();
        let proposal = Proposal::new(committee, header.clone(), transmission_map.clone()).unwrap();
        assert_eq!(proposal.batch_id(), header.batch_id());
    }
}
