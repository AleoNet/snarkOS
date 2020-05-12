use crate::storage::Storage;
use snarkos_errors::algorithms::SignatureError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait SignatureScheme: Sized + Clone + Storage {
    type Parameters: Clone + ToBytes + FromBytes + Send + Sync;
    type PublicKey: ToBytes + FromBytes + Hash + Eq + Clone + Debug + Default + Send + Sync;
    type PrivateKey: ToBytes + FromBytes + PartialEq + Eq + Clone + Default + Debug;
    type Output: ToBytes + FromBytes + Clone + Debug + Default + Send + Sync;

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
