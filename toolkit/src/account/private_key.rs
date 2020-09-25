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

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkos_models::{algorithms::SignatureScheme, dpc::DPCComponents};
use snarkos_objects::AccountPrivateKey;
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

use rand::{CryptoRng, Rng};
use std::{fmt, str::FromStr};

pub struct Signature(pub <<Components as DPCComponents>::AccountSignature as SignatureScheme>::Output);

impl FromStr for Signature {
    type Err = PrivateKeyError;

    fn from_str(signature: &str) -> Result<Self, Self::Err> {
        let signature_bytes = hex::decode(signature)?;
        let signature: <<Components as DPCComponents>::AccountSignature as SignatureScheme>::Output =
            FromBytes::read(&signature_bytes[..])?;

        Ok(Self(signature))
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            hex::encode(to_bytes![self.0].expect("failed to convert to bytes"))
        )
    }
}

pub struct SignaturePublicKey(pub <<Components as DPCComponents>::AccountSignature as SignatureScheme>::PublicKey);

impl FromStr for SignaturePublicKey {
    type Err = PrivateKeyError;

    fn from_str(public_key: &str) -> Result<Self, Self::Err> {
        let public_key_bytes = hex::decode(public_key)?;
        let public_key: <<Components as DPCComponents>::AccountSignature as SignatureScheme>::PublicKey =
            FromBytes::read(&public_key_bytes[..])?;

        Ok(Self(public_key))
    }
}

impl fmt::Display for SignaturePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            hex::encode(to_bytes![self.0].expect("failed to convert to bytes"))
        )
    }
}

#[derive(Debug)]
pub struct PrivateKey {
    pub(crate) private_key: AccountPrivateKey<Components>,
}

impl PrivateKey {
    /// Returns a new Account Private Key.
    pub fn new<R: Rng + CryptoRng>(rng: &mut R) -> Result<Self, PrivateKeyError> {
        let parameters = SystemParameters::<Components>::load()?;
        let private_key =
            AccountPrivateKey::<Components>::new(&parameters.account_signature, &parameters.account_commitment, rng)?;
        Ok(Self { private_key })
    }

    /// Sign a message with the private key `sk_sig`
    pub fn sign<R: Rng + CryptoRng>(&self, message: &[u8], rng: &mut R) -> Result<Signature, PrivateKeyError> {
        let parameters = SystemParameters::<Components>::load()?;

        let signature = parameters
            .account_signature
            .sign(&self.private_key.sk_sig, message, rng)?;

        Ok(Signature(signature))
    }

    /// Returns the signature public key `pk_sig` used to verify signed messages.
    pub fn to_signature_public_key(&self) -> Result<SignaturePublicKey, PrivateKeyError> {
        let parameters = SystemParameters::<Components>::load()?;

        let public_key = self.private_key.pk_sig(&parameters.account_signature)?;

        Ok(SignaturePublicKey(public_key))
    }

    /// Verify a signature signed by the private key
    /// Returns `true` if the signature is verified correctly. Otherwise, returns `false`.
    pub fn verify(
        public_key: &SignaturePublicKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool, PrivateKeyError> {
        let parameters = SystemParameters::<Components>::load()?;

        Ok(parameters
            .account_signature
            .verify(&public_key.0, message, &signature.0)?)
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
