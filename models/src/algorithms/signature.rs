use snarkos_errors::algorithms::SignatureError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::{hash::Hash, path::PathBuf};

pub trait SignatureScheme: Sized {
    type Parameters: Clone + ToBytes + FromBytes + Send + Sync;
    type PublicKey: ToBytes + FromBytes + Hash + Eq + Clone + Default + Send + Sync;
    type PrivateKey: ToBytes + Clone + Default;
    type Output: ToBytes + FromBytes + Clone + Default + Send + Sync;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self, SignatureError>;

    fn keygen<R: Rng>(&self, rng: &mut R) -> Result<(Self::PublicKey, Self::PrivateKey), SignatureError>;

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

    fn store(&self, path: &PathBuf) -> Result<(), SignatureError>;

    fn load(path: &PathBuf) -> Result<Self, SignatureError>;
}
