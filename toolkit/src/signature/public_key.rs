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

use crate::{account::PrivateKey, errors::SignatureError};

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkos_models::{algorithms::SignatureScheme, dpc::DPCComponents};
use snarkos_utilities::bytes::ToBytes;

use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct PublicKey {
    pub(crate) public_key: <<Components as DPCComponents>::AccountSignature as SignatureScheme>::PublicKey,
}

impl PublicKey {
    pub fn from_private_key(private_key: &PrivateKey) -> Result<Self, SignatureError> {
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

impl FromStr for PublicKey {
    type Err = SignatureError;

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        unimplemented!()
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}
