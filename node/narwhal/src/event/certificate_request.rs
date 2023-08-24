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
pub struct CertificateRequest<N: Network> {
    pub certificate_id: Field<N>,
}

impl<N: Network> CertificateRequest<N> {
    /// Initializes a new certificate request event.
    pub const fn new(certificate_id: Field<N>) -> Self {
        Self { certificate_id }
    }
}

impl<N: Network> From<Field<N>> for CertificateRequest<N> {
    /// Initializes a new certificate request event.
    fn from(certificate_id: Field<N>) -> Self {
        Self::new(certificate_id)
    }
}

impl<N: Network> EventTrait for CertificateRequest<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "CertificateRequest"
    }
}

impl<N: Network> ToBytes for CertificateRequest<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.certificate_id.write_le(&mut writer)?;
        Ok(())
    }
}

impl<N: Network> FromBytes for CertificateRequest<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let certificate_id = Field::read_le(&mut reader)?;

        Ok(Self { certificate_id })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{helpers::storage::prop_tests::CryptoTestRng, CertificateRequest};
    use bytes::{Buf, BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use snarkvm::{
        console::prelude::{FromBytes, ToBytes},
        prelude::{Field, Uniform},
    };
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    pub fn any_field() -> BoxedStrategy<Field<CurrentNetwork>> {
        any::<CryptoTestRng>().prop_map(|mut rng| Field::rand(&mut rng)).boxed()
    }

    pub fn any_certificate_request() -> BoxedStrategy<CertificateRequest<CurrentNetwork>> {
        any_field().prop_map(CertificateRequest::new).boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_certificate_request())] original: CertificateRequest<CurrentNetwork>) {
        let mut buf = BytesMut::default().writer();
        CertificateRequest::write_le(&original, &mut buf).unwrap();

        let deserialized: CertificateRequest<CurrentNetwork> =
            CertificateRequest::read_le(buf.into_inner().reader()).unwrap();
        assert_eq!(original, deserialized);
    }
}