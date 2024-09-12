// Copyright 2024 Aleo Network Foundation
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
pub struct ValidatorsResponse<N: Network> {
    pub validators: IndexMap<SocketAddr, Address<N>>,
}

impl<N: Network> EventTrait for ValidatorsResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "ValidatorsResponse".into()
    }
}

impl<N: Network> ToBytes for ValidatorsResponse<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the number of validators.
        u16::try_from(self.validators.len()).map_err(error)?.write_le(&mut writer)?;
        // Write the validators.
        for (socket_addr, address) in &self.validators {
            socket_addr.write_le(&mut writer)?;
            address.write_le(&mut writer)?;
        }
        Ok(())
    }
}

impl<N: Network> FromBytes for ValidatorsResponse<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the number of validators.
        let num_validators = u16::read_le(&mut reader)?;
        // Read the validators.
        let mut validators = IndexMap::with_capacity(num_validators as usize);
        for _ in 0..num_validators {
            let socket_addr = SocketAddr::read_le(&mut reader)?;
            let address = Address::<N>::read_le(&mut reader)?;
            validators.insert(socket_addr, address);
        }
        Ok(Self { validators })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{challenge_request::prop_tests::any_valid_address, ValidatorsResponse};

    use bytes::{Buf, BufMut, BytesMut};
    use indexmap::IndexMap;
    use proptest::{
        collection::hash_map,
        prelude::{any, BoxedStrategy, Strategy},
    };
    use snarkvm::{
        prelude::Address,
        utilities::{FromBytes, ToBytes},
    };
    use std::net::{IpAddr, SocketAddr};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    pub fn any_valid_socket_addr() -> BoxedStrategy<SocketAddr> {
        any::<(IpAddr, u16)>().prop_map(|(ip_addr, port)| SocketAddr::new(ip_addr, port)).boxed()
    }

    pub fn any_index_map() -> BoxedStrategy<IndexMap<SocketAddr, Address<CurrentNetwork>>> {
        hash_map(any_valid_socket_addr(), any_valid_address(), 0..50)
            .prop_map(|map| map.iter().map(|(k, v)| (*k, *v)).collect())
            .boxed()
    }

    pub fn any_validators_response() -> BoxedStrategy<ValidatorsResponse<CurrentNetwork>> {
        any_index_map().prop_map(|map| ValidatorsResponse { validators: map }).boxed()
    }

    #[proptest]
    fn validators_response_roundtrip(
        #[strategy(any_validators_response())] validators_response: ValidatorsResponse<CurrentNetwork>,
    ) {
        let mut bytes = BytesMut::default().writer();
        validators_response.write_le(&mut bytes).unwrap();
        let decoded = ValidatorsResponse::<CurrentNetwork>::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq![decoded, validators_response];
    }
}
