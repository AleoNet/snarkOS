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
pub struct CertificateResponse<N: Network> {
    pub certificate: BatchCertificate<N>,
}

impl<N: Network> CertificateResponse<N> {
    /// Initializes a new certificate response event.
    pub fn new(certificate: BatchCertificate<N>) -> Self {
        Self { certificate }
    }
}

impl<N: Network> From<BatchCertificate<N>> for CertificateResponse<N> {
    /// Initializes a new certificate response event.
    fn from(certificate: BatchCertificate<N>) -> Self {
        Self::new(certificate)
    }
}

impl<N: Network> EventTrait for CertificateResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "CertificateResponse"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.certificate.to_bytes_le()?)?;
        Ok(())
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let certificate = BatchCertificate::read_le(&mut reader)?;

        Ok(Self { certificate })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{
        event::{transmission_response::prop_tests::any_transmission, EventTrait},
        helpers::{
            now,
            storage::prop_tests::{sign_batch_header, CryptoTestRng},
        },
        CertificateResponse,
    };
    use bytes::{BufMut, BytesMut};
    use proptest::{
        collection::vec,
        prelude::{any, BoxedStrategy, Just, Strategy},
        sample::Selector,
    };
    use snarkos_node_narwhal_committee::prop_tests::{CommitteeContext, ValidatorSet};
    use snarkvm::ledger::narwhal::{BatchCertificate, BatchHeader};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    pub fn any_batch_header(committee: &CommitteeContext) -> BoxedStrategy<BatchHeader<CurrentNetwork>> {
        (Just(committee.clone()), any::<Selector>(), any::<CryptoTestRng>(), vec(any_transmission(), 0..16))
            .prop_map(|(committee, selector, mut rng, transmissions)| {
                let CommitteeContext(_, ValidatorSet(validators)) = committee;
                let signer = selector.select(validators);
                let transmission_ids = transmissions.into_iter().map(|(id, _)| id).collect();

                BatchHeader::new(signer.account.private_key(), 0, now(), transmission_ids, Default::default(), &mut rng)
                    .unwrap()
            })
            .boxed()
    }

    pub fn any_batch_certificate() -> BoxedStrategy<BatchCertificate<CurrentNetwork>> {
        any::<CommitteeContext>()
            .prop_flat_map(|committee| (Just(committee.clone()), any_batch_header(&committee), any::<CryptoTestRng>()))
            .prop_map(|(committee, batch_header, mut rng)| {
                let CommitteeContext(_, validator_set) = committee;
                BatchCertificate::new(batch_header.clone(), sign_batch_header(&validator_set, &batch_header, &mut rng))
                    .unwrap()
            })
            .boxed()
    }

    pub fn any_certificate_response() -> BoxedStrategy<CertificateResponse<CurrentNetwork>> {
        any_batch_certificate().prop_map(CertificateResponse::new).boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_certificate_response())] original: CertificateResponse<CurrentNetwork>) {
        let mut buf = BytesMut::default().writer();
        CertificateResponse::serialize(&original, &mut buf).unwrap();

        let deserialized: CertificateResponse<CurrentNetwork> =
            CertificateResponse::deserialize(buf.get_ref().clone()).unwrap();
        assert_eq!(original, deserialized);
    }
}
