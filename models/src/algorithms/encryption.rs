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

use snarkos_errors::algorithms::EncryptionError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait EncryptionScheme: Sized + Clone + From<<Self as EncryptionScheme>::Parameters> {
    type Parameters: Clone + Debug + Eq + ToBytes + FromBytes;
    type PrivateKey: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes + UniformRand;
    type PublicKey: Clone + Debug + Default + Eq + ToBytes + FromBytes;
    type Text: Clone + Debug + Default + Eq + ToBytes + FromBytes;
    type Randomness: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes + UniformRand;
    type BlindingExponent: Clone + Debug + Default + Eq + Hash + ToBytes;

    fn setup<R: Rng>(rng: &mut R) -> Self;

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Self::PrivateKey;

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Result<Self::PublicKey, EncryptionError>;

    fn generate_randomness<R: Rng>(
        &self,
        public_key: &Self::PublicKey,
        rng: &mut R,
    ) -> Result<Self::Randomness, EncryptionError>;

    fn generate_blinding_exponents(
        &self,
        public_key: &Self::PublicKey,
        randomness: &Self::Randomness,
        message_length: usize,
    ) -> Result<Vec<Self::BlindingExponent>, EncryptionError>;

    fn encrypt(
        &self,
        public_key: &Self::PublicKey,
        randomness: &Self::Randomness,
        message: &[Self::Text],
    ) -> Result<Vec<Self::Text>, EncryptionError>;

    fn decrypt(
        &self,
        private_key: &Self::PrivateKey,
        ciphertext: &[Self::Text],
    ) -> Result<Vec<Self::Text>, EncryptionError>;

    fn parameters(&self) -> &Self::Parameters;

    fn private_key_size_in_bits() -> usize;
}
