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
    signature::{SchnorrOutput, SchnorrParameters, SchnorrPublicKey},
};
use snarkos_errors::{algorithms::SignatureError, curves::ConstraintFieldError, serialization::SerializationError};
use snarkos_models::{
    algorithms::{EncryptionScheme, SignatureScheme},
    curves::{to_field_vec::ToConstraintField, Field, Group, One, PrimeField, ProjectiveCurve, Zero},
};

use snarkos_utilities::{
    bytes::{bytes_to_bits, FromBytes, ToBytes},
    rand::UniformRand,
    serialize::*,
    to_bytes,
};

use crate::signature::SchnorrSignature;
use digest::Digest;
use itertools::Itertools;
use rand::Rng;
use std::{
    hash::Hash,
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
};

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

impl<G: Group + ProjectiveCurve + Hash + CanonicalSerialize + CanonicalDeserialize> SignatureScheme
    for GroupEncryption<G>
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
        // let schnorr_parameters: SchnorrSignature<G, _> = self.parameters.into();

        // SchnorrSignature::<G, _>::sign()
        unimplemented!()
    }

    fn verify(
        &self,
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Output,
    ) -> Result<bool, SignatureError> {
        unimplemented!()
    }

    fn randomize_public_key(
        &self,
        public_key: &Self::PublicKey,
        randomness: &[u8],
    ) -> Result<Self::PublicKey, SignatureError> {
        unimplemented!()
    }

    fn randomize_signature(&self, signature: &Self::Output, randomness: &[u8]) -> Result<Self::Output, SignatureError> {
        unimplemented!()
    }
}
