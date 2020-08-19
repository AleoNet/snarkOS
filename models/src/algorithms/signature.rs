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

use snarkos_errors::algorithms::SignatureError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    serialize::{CanonicalDeserialize, CanonicalSerialize},
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait SignatureScheme: Sized + Clone + From<<Self as SignatureScheme>::Parameters> {
    type Parameters: Clone + Debug + ToBytes + FromBytes + Eq + Send + Sync;
    type PublicKey: Clone
        + Debug
        + Default
        + ToBytes
        + FromBytes
        + Hash
        + Eq
        + Send
        + Sync
        + CanonicalSerialize
        + CanonicalDeserialize;
    type PrivateKey: Clone + Debug + Default + ToBytes + FromBytes + PartialEq + Eq;
    type Output: Clone + Debug + Default + ToBytes + FromBytes + Send + Sync;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self, SignatureError>;

    fn parameters(&self) -> &Self::Parameters;

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Result<Self::PrivateKey, SignatureError>;

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Result<Self::PublicKey, SignatureError>;

    fn sign<R: Rng>(
        &self,
        private_key: &Self::PrivateKey,
        message: &[u8],
        rng: &mut R,
    ) -> Result<Self::Output, SignatureError>;

    fn verify(
        &self,
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Output,
    ) -> Result<bool, SignatureError>;

    fn randomize_public_key(
        &self,
        public_key: &Self::PublicKey,
        randomness: &[u8],
    ) -> Result<Self::PublicKey, SignatureError>;

    fn randomize_signature(&self, signature: &Self::Output, randomness: &[u8]) -> Result<Self::Output, SignatureError>;
}
