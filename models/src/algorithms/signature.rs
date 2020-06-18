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
