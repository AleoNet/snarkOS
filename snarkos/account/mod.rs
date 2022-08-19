// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use snarkvm::{console::types::Field, prelude::*};

use ::rand::thread_rng;

/// A helper struct for an Aleo account.
#[derive(Debug)]
pub struct Account<N: Network> {
    /// The account private key.
    private_key: PrivateKey<N>,
    /// The account view key.
    view_key: ViewKey<N>,
    /// The account address.
    address: Address<N>,
}

impl<N: Network> FromStr for Account<N> {
    type Err = anyhow::Error;

    /// Initializes a new account from a private key string.
    fn from_str(private_key: &str) -> Result<Self, Self::Err> {
        Self::new(FromStr::from_str(private_key)?)
    }
}

impl<N: Network> Account<N> {
    /// Initializes a new account.
    pub fn new(private_key: PrivateKey<N>) -> Result<Self> {
        Ok(Self {
            private_key,
            view_key: ViewKey::try_from(&private_key)?,
            address: Address::try_from(&private_key)?,
        })
    }

    /// Samples a new account.
    pub fn sample() -> Result<Self> {
        Self::new(PrivateKey::new(&mut thread_rng())?)
    }

    /// Signs a given message.
    pub fn sign(&self, message: &[Field<N>]) -> Result<Signature<N>> {
        Signature::sign(&self.private_key, message, &mut thread_rng())
    }

    /// Verifies a given message and signature.
    pub fn verify(&self, message: &[Field<N>], signature: &Signature<N>) -> bool {
        signature.verify(&self.address, message)
    }

    /// Returns the account private key.
    pub const fn private_key(&self) -> &PrivateKey<N> {
        &self.private_key
    }

    /// Returns the account view key.
    pub const fn view_key(&self) -> &ViewKey<N> {
        &self.view_key
    }

    /// Returns the account address.
    pub const fn address(&self) -> &Address<N> {
        &self.address
    }
}
