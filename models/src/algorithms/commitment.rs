use crate::storage::Storage;
use snarkos_errors::algorithms::CommitmentError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait CommitmentScheme: Sized + Clone + Storage {
    type Output: ToBytes + FromBytes + Clone + Default + Eq + Hash + Debug;
    type Parameters: Clone;
    type Randomness: Clone + ToBytes + FromBytes + Default + Eq + UniformRand + Debug;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError>;

    fn parameters(&self) -> &Self::Parameters;
}
