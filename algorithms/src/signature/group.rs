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
use snarkos_utilities::{serialize::*, to_bytes, FromBytes, ToBytes};

use digest::Digest;
use rand::Rng;
use std::{hash::Hash, marker::PhantomData};

/// Map the encryption group into the signature group
fn into_signature_group<G: Group + ProjectiveCurve, SG: Group>(projective: G) -> SG {
    let bytes = to_bytes![&projective.into_affine()].expect("failed to convert to bytes");
    FromBytes::read(&bytes[..]).expect("failed to convert to signature group")
}

impl<G: Group + ProjectiveCurve, SG: Group, D: Digest> From<GroupEncryptionParameters<G>> for SchnorrSignature<SG, D> {
    fn from(parameters: GroupEncryptionParameters<G>) -> Self {
        let generator_powers: Vec<SG> = parameters
            .generator_powers
            .iter()
            .map(|p| into_signature_group(*p))
            .collect();

        let parameters = SchnorrParameters {
            generator_powers,
            salt: parameters.salt,
            _hash: PhantomData,
        };

        Self { parameters }
    }
}

impl<G: Group + ProjectiveCurve, SG: Group + CanonicalSerialize + CanonicalDeserialize>
    From<GroupEncryptionPublicKey<G>> for SchnorrPublicKey<SG>
{
    fn from(public_key: GroupEncryptionPublicKey<G>) -> Self {
        Self(into_signature_group(public_key.0))
    }
}

impl<G: Group + ProjectiveCurve, SG: Group + Hash + CanonicalSerialize + CanonicalDeserialize, D: Digest + Send + Sync>
    SignatureScheme for GroupEncryption<G, SG, D>
where
    <G as Group>::ScalarField: PrimeField,
{
    type Output = SchnorrOutput<SG>;
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
        let schnorr_signature: SchnorrSignature<SG, D> = self.parameters.clone().into();
        let private_key = <SG as Group>::ScalarField::read(&to_bytes![private_key]?[..])?;

        Ok(schnorr_signature.sign(&private_key, message, rng)?)
    }

    fn verify(
        &self,
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Output,
    ) -> Result<bool, SignatureError> {
        let schnorr_signature: SchnorrSignature<SG, D> = self.parameters.clone().into();
        let schnorr_public_key: SchnorrPublicKey<SG> = (*public_key).into();

        Ok(schnorr_signature.verify(&schnorr_public_key, message, signature)?)
    }

    fn randomize_public_key(
        &self,
        _public_key: &Self::PublicKey,
        _randomness: &[u8],
    ) -> Result<Self::PublicKey, SignatureError> {
        unimplemented!()
    }

    fn randomize_signature(
        &self,
        _signature: &Self::Output,
        _randomness: &[u8],
    ) -> Result<Self::Output, SignatureError> {
        unimplemented!()
    }
}
