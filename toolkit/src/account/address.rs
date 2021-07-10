// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{account::PrivateKey, errors::AddressError};

use snarkvm_dpc::{
    testnet1::{instantiated::Components, parameters::SystemParameters},
    Address as DPCAddress,
};
use snarkvm_utilities::bytes::ToBytes;

use std::{fmt, str::FromStr};

#[derive(Clone, Debug)]
pub struct Address {
    pub(crate) address: DPCAddress<Components>,
}

impl Address {
    pub fn from(private_key: &PrivateKey) -> Result<Self, AddressError> {
        let parameters = SystemParameters::<Components>::load()?;
        let address = DPCAddress::<Components>::from_private_key(
            &parameters.account_signature,
            &parameters.account_commitment,
            &parameters.account_encryption,
            &private_key.private_key,
        )?;
        Ok(Self { address })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = vec![];
        self.address.write(&mut output).expect("serialization to bytes failed");
        output
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            address: DPCAddress::<Components>::from_str(address)?,
        })
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.address.to_string())
    }
}
