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

use crate::account::PrivateKey;
use crate::errors::SignatureError;

use snarkvm_dpc::base_dpc::instantiated::Components;
use snarkvm_dpc::base_dpc::parameters::SystemParameters;
use snarkvm_algorithms::SignatureScheme;
use snarkvm_dpc::DPCComponents;
use snarkvm_utilities::bytes::FromBytes;
use snarkvm_utilities::bytes::ToBytes;
use snarkvm_utilities::to_bytes;

use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct SignaturePublicKey {
    pub(crate) public_key: <<Components as DPCComponents>::AccountSignature as SignatureScheme>::PublicKey,
}

impl SignaturePublicKey {
    pub fn from(private_key: &PrivateKey) -> Result<Self, SignatureError> {
        let parameters = SystemParameters::<Components>::load()?;

        let public_key = private_key.private_key.pk_sig(&parameters.account_signature)?;

        Ok(Self { public_key })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = vec![];
        self.public_key
            .write(&mut output)
            .expect("serialization to bytes failed");
        output
    }
}

impl FromStr for SignaturePublicKey {
    type Err = SignatureError;

    fn from_str(public_key: &str) -> Result<Self, Self::Err> {
        let public_key_bytes = hex::decode(public_key)?;
        let public_key: <<Components as DPCComponents>::AccountSignature as SignatureScheme>::PublicKey =
            FromBytes::read(&public_key_bytes[..])?;

        Ok(Self { public_key })
    }
}

impl fmt::Display for SignaturePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            hex::encode(to_bytes![self.public_key].expect("failed to convert to bytes"))
        )
    }
}
