use snarkos_errors::algorithms::EncryptionError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait EncryptionScheme: Sized + Clone {
    type PrivateKey: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes + UniformRand;
    type PublicKey: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes;
    type Plaintext: Clone + Debug + Default + Eq + Hash;
    type Ciphertext: Clone + Debug + Default + Eq + Hash;

    fn setup<R: Rng>(rng: &mut R) -> Self;

    fn keygen<R: Rng>(&self, rng: &mut R) -> (Self::PrivateKey, Self::PublicKey);

    fn encrypt<R: Rng>(
        &self,
        public_key: &Self::PublicKey,
        message: &Self::Plaintext,
        rng: &mut R,
    ) -> Result<Self::Ciphertext, EncryptionError>;

    fn decrypt(
        &self,
        private_key: &Self::PrivateKey,
        ciphertext: &Self::Ciphertext,
    ) -> Result<Self::Plaintext, EncryptionError>;
}
