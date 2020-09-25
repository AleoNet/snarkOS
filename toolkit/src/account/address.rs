// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::{
    account::{
        view_key::{Signature as ViewKeySignature, ViewKey},
        PrivateKey,
    },
    errors::AddressError,
};

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkos_models::algorithms::signature::SignatureScheme;
use snarkos_objects::AccountAddress;
use snarkos_utilities::bytes::ToBytes;

use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct Address {
    pub(crate) address: AccountAddress<Components>,
}

impl Address {
    pub fn from(private_key: &PrivateKey) -> Result<Self, AddressError> {
        let parameters = SystemParameters::<Components>::load()?;
        let address = AccountAddress::<Components>::from_private_key(
            &parameters.account_signature,
            &parameters.account_commitment,
            &parameters.account_encryption,
            &private_key.private_key,
        )?;
        Ok(Self { address })
    }

    pub fn from_view_key(view_key: &ViewKey) -> Result<Self, AddressError> {
        let parameters = SystemParameters::<Components>::load()?;
        let address = AccountAddress::<Components>::from_view_key(&parameters.account_encryption, &view_key.view_key)?;
        Ok(Self { address })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = vec![];
        self.address.write(&mut output).expect("serialization to bytes failed");
        output
    }

    /// Verify a signature signed by the view key
    /// Returns `true` if the signature is verified correctly. Otherwise, returns `false`.
    pub fn verify(&self, message: &[u8], signature: ViewKeySignature) -> Result<bool, AddressError> {
        let parameters = SystemParameters::<Components>::load()?;

        Ok(parameters
            .account_encryption
            .verify(&self.address.encryption_key, message, &signature.0)?)
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            address: AccountAddress::<Components>::from_str(address)?,
        })
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.address.to_string())
    }
}
