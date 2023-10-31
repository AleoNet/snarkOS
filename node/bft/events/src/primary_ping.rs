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

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimaryPing<N: Network> {
    pub version: u32,
    pub block_locators: BlockLocators<N>,
    pub primary_certificate: Data<BatchCertificate<N>>,
    pub batch_certificates: IndexMap<Field<N>, Data<BatchCertificate<N>>>,
}

impl<N: Network> PrimaryPing<N> {
    /// Initializes a new ping event.
    pub const fn new(
        version: u32,
        block_locators: BlockLocators<N>,
        primary_certificate: Data<BatchCertificate<N>>,
        batch_certificates: IndexMap<Field<N>, Data<BatchCertificate<N>>>,
    ) -> Self {
        Self { version, block_locators, primary_certificate, batch_certificates }
    }
}

impl<N: Network> From<(u32, BlockLocators<N>, BatchCertificate<N>, IndexSet<BatchCertificate<N>>)> for PrimaryPing<N> {
    /// Initializes a new ping event.
    fn from(
        (version, block_locators, primary_certificate, batch_certificates): (
            u32,
            BlockLocators<N>,
            BatchCertificate<N>,
            IndexSet<BatchCertificate<N>>,
        ),
    ) -> Self {
        Self::new(
            version,
            block_locators,
            Data::Object(primary_certificate),
            batch_certificates.into_iter().map(|c| (c.id(), Data::Object(c))).collect(),
        )
    }
}

impl<N: Network> EventTrait for PrimaryPing<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "PrimaryPing".into()
    }
}

impl<N: Network> ToBytes for PrimaryPing<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        self.version.write_le(&mut writer)?;
        // Write the block locators.
        self.block_locators.write_le(&mut writer)?;
        // Write the primary certificate.
        self.primary_certificate.write_le(&mut writer)?;

        // Determine the number of batch certificates.
        let num_certificates =
            u16::try_from(self.batch_certificates.len()).map_err(error)?.min(Committee::<N>::MAX_COMMITTEE_SIZE);

        // Write the number of batch certificates.
        num_certificates.write_le(&mut writer)?;
        // Write the batch certificates.
        for (certificate_id, certificate) in self.batch_certificates.iter().take(usize::from(num_certificates)) {
            certificate_id.write_le(&mut writer)?;
            certificate.write_le(&mut writer)?;
        }
        Ok(())
    }
}

impl<N: Network> FromBytes for PrimaryPing<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u32::read_le(&mut reader)?;
        // Read the block locators.
        let block_locators = BlockLocators::read_le(&mut reader)?;
        // Read the primary certificate.
        let primary_certificate = Data::read_le(&mut reader)?;

        // Read the number of batch certificates.
        let num_certificates = u16::read_le(&mut reader)?;
        // Ensure the number of batch certificates is not greater than the maximum committee size.
        // Note: We allow there to be 0 batch certificates. This is necessary to ensure primary pings are sent.
        if num_certificates > Committee::<N>::MAX_COMMITTEE_SIZE {
            return Err(error("The number of batch certificates is greater than the maximum committee size"));
        }

        // Read the batch certificates.
        let mut batch_certificates = IndexMap::with_capacity(usize::from(num_certificates));
        for _ in 0..num_certificates {
            // Read the certificate ID.
            let certificate_id = Field::read_le(&mut reader)?;
            // Read the certificate.
            let certificate = Data::read_le(&mut reader)?;
            // Insert the certificate.
            batch_certificates.insert(certificate_id, certificate);
        }

        // Return the ping event.
        Ok(Self::new(version, block_locators, primary_certificate, batch_certificates))
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{certificate_response::prop_tests::any_batch_certificate, PrimaryPing};
    use snarkos_node_sync_locators::{test_helpers::sample_block_locators, BlockLocators};
    use snarkvm::utilities::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};
    use indexmap::indexset;
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    pub fn any_block_locators() -> BoxedStrategy<BlockLocators<CurrentNetwork>> {
        any::<u32>().prop_map(sample_block_locators).boxed()
    }

    pub fn any_primary_ping() -> BoxedStrategy<PrimaryPing<CurrentNetwork>> {
        (any::<u32>(), any_block_locators(), any_batch_certificate())
            .prop_map(|(version, block_locators, batch_certificate)| {
                PrimaryPing::from((version, block_locators, batch_certificate.clone(), indexset![batch_certificate]))
            })
            .boxed()
    }

    #[proptest]
    fn primary_ping_roundtrip(#[strategy(any_primary_ping())] primary_ping: PrimaryPing<CurrentNetwork>) {
        let mut bytes = BytesMut::default().writer();
        primary_ping.write_le(&mut bytes).unwrap();
        let decoded = PrimaryPing::<CurrentNetwork>::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq!(primary_ping.version, decoded.version);
        assert_eq!(primary_ping.block_locators, decoded.block_locators);
        assert_eq!(
            primary_ping.primary_certificate.deserialize_blocking().unwrap(),
            decoded.primary_certificate.deserialize_blocking().unwrap(),
        );
        assert!(
            primary_ping
                .batch_certificates
                .into_iter()
                .map(|(a, bc)| (a, bc.deserialize_blocking().unwrap()))
                .zip(decoded.batch_certificates.into_iter().map(|(a, bc)| (a, bc.deserialize_blocking().unwrap())))
                .all(|(a, b)| a == b)
        )
    }
}
