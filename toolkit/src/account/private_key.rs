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

use crate::errors::PrivateKeyError;

use snarkvm_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkvm_objects::AccountPrivateKey;

use rand::{CryptoRng, Rng};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug)]
pub struct PrivateKey {
    pub(crate) private_key: AccountPrivateKey<Components>,
}

impl PrivateKey {
    pub fn new<R: Rng + CryptoRng>(rng: &mut R) -> Result<Self, PrivateKeyError> {
        let parameters = SystemParameters::<Components>::load()?;
        let private_key =
            AccountPrivateKey::<Components>::new(&parameters.account_signature, &parameters.account_commitment, rng)?;
        Ok(Self { private_key })
    }
}

impl FromStr for PrivateKey {
    type Err = PrivateKeyError;

    fn from_str(private_key: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            private_key: AccountPrivateKey::<Components>::from_str(private_key)?,
        })
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.private_key.to_string())
    }
}
