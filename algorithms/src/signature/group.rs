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
    encryption::{GroupEncryption, GroupEncryptionParameters, GroupEncryptionPublicKey},
    signature::{SchnorrOutput, SchnorrParameters, SchnorrPublicKey, SchnorrSignature},
};
use snarkos_errors::algorithms::SignatureError;
use snarkos_models::{
    algorithms::{EncryptionScheme, SignatureScheme},
    curves::{Group, PrimeField, ProjectiveCurve},
};
use snarkos_utilities::serialize::*;

use digest::Digest;
use rand::Rng;
use std::{hash::Hash, marker::PhantomData};

impl<G: Group + ProjectiveCurve, D: Digest> From<GroupEncryptionParameters<G>> for SchnorrSignature<G, D> {
    fn from(parameters: GroupEncryptionParameters<G>) -> Self {
        let parameters = SchnorrParameters {
            generator_powers: parameters.generator_powers,
            salt: parameters.salt,
            _hash: PhantomData,
        };

        Self { parameters }
    }
}

impl<G: Group + ProjectiveCurve> From<GroupEncryptionPublicKey<G>> for SchnorrPublicKey<G> {
    fn from(public_key: GroupEncryptionPublicKey<G>) -> Self {
        Self(public_key.0)
    }
}

impl<G: Group + ProjectiveCurve> From<SchnorrPublicKey<G>> for GroupEncryptionPublicKey<G> {
    fn from(public_key: SchnorrPublicKey<G>) -> Self {
        Self(public_key.0)
    }
}

impl<G: Group + ProjectiveCurve + Hash + CanonicalSerialize + CanonicalDeserialize, D: Digest + Send + Sync>
    SignatureScheme for GroupEncryption<G, D>
where
    <G as Group>::ScalarField: PrimeField,
{
    type Output = SchnorrOutput<G>;
    type Parameters = GroupEncryptionParameters<G>;
    type PrivateKey = <G as Group>::ScalarField;
    type PublicKey = GroupEncryptionPublicKey<G>;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self, SignatureError> {
        Ok(<Self as EncryptionScheme>::setup(rng))
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Result<Self::PrivateKey, SignatureError> {
        Ok(<Self as EncryptionScheme>::generate_private_key(self, rng))
    }

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Result<Self::PublicKey, SignatureError> {
        Ok(<Self as EncryptionScheme>::generate_public_key(self, private_key).unwrap())
    }

    fn sign<R: Rng>(
        &self,
        private_key: &Self::PrivateKey,
        message: &[u8],
        rng: &mut R,
    ) -> Result<Self::Output, SignatureError> {
        let schnorr_signature: SchnorrSignature<G, D> = self.parameters.clone().into();

        Ok(schnorr_signature.sign(private_key, message, rng)?)
    }

    fn verify(
        &self,
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Output,
    ) -> Result<bool, SignatureError> {
        let schnorr_signature: SchnorrSignature<G, D> = self.parameters.clone().into();
        let schnorr_public_key: SchnorrPublicKey<G> = public_key.clone().into();

        Ok(schnorr_signature.verify(&schnorr_public_key, message, signature)?)
    }

    fn randomize_public_key(
        &self,
        public_key: &Self::PublicKey,
        randomness: &[u8],
    ) -> Result<Self::PublicKey, SignatureError> {
        let schnorr_signature: SchnorrSignature<G, D> = self.parameters.clone().into();
        let schnorr_public_key: SchnorrPublicKey<G> = public_key.clone().into();

        Ok(schnorr_signature
            .randomize_public_key(&schnorr_public_key, randomness)?
            .into())
    }

    fn randomize_signature(&self, signature: &Self::Output, randomness: &[u8]) -> Result<Self::Output, SignatureError> {
        let schnorr_signature: SchnorrSignature<G, D> = self.parameters.clone().into();

        Ok(schnorr_signature.randomize_signature(&signature, randomness)?)
    }
}
