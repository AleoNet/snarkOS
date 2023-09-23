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
