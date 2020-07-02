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

    fn setup<R: Rng>(rng: &mut R) -> Self;

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Self::PrivateKey;

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Self::PublicKey;

    // TODO (raychu86) clean up model for returning randomness and blinding exponents
    fn encrypt<R: Rng>(
        &self,
        public_key: &Self::PublicKey,
        message: &Self::Plaintext,
        rng: &mut R,
    ) -> Result<(Self::Ciphertext, Self::Randomness, Vec<Self::Randomness>), EncryptionError>;

    fn decrypt(
        &self,
        private_key: &Self::PrivateKey,
        ciphertext: &Self::Ciphertext,
    ) -> Result<Self::Plaintext, EncryptionError>;

    fn parameters(&self) -> &Self::Parameters;
}
