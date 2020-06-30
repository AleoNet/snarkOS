use snarkos_errors::algorithms::EncryptionError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait EncryptionScheme: Sized + Clone {
    type Output: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes;
    type Randomness: Clone + Debug + Default + Eq + UniformRand + ToBytes + FromBytes;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn encrypt(&self, message: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, EncryptionError>;

    fn decrypt(&self, ciphertext: &Self::Output) -> Result<Vec<u8>, EncryptionError>;
}
