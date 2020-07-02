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
    type PublicKey: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes;
    type Plaintext: Clone + Debug + Default + Eq + Hash;
    type Ciphertext: Clone + Debug + Default + Eq + Hash;
    type Randomness: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes + UniformRand;
    type BlindingExponents: Clone + Debug + Default + Eq + Hash + ToBytes;

    fn setup<R: Rng>(rng: &mut R) -> Self;

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Self::PrivateKey;

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Self::PublicKey;

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
    ) -> Result<Self::BlindingExponents, EncryptionError>;

    fn encrypt(
        &self,
        public_key: &Self::PublicKey,
        randomness: &Self::Randomness,
        message: &Self::Plaintext,
    ) -> Result<Self::Ciphertext, EncryptionError>;

    fn decrypt(
        &self,
        private_key: &Self::PrivateKey,
        ciphertext: &Self::Ciphertext,
    ) -> Result<Self::Plaintext, EncryptionError>;

    fn parameters(&self) -> &Self::Parameters;
}
