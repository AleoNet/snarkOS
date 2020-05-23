use snarkos_errors::algorithms::CommitmentError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait CommitmentScheme: Sized + Clone {
    type Output: Clone + Debug + Default + Eq + Hash + ToBytes + FromBytes;
    type Parameters: Clone + Debug + Eq + ToBytes + FromBytes;
    type Randomness: Clone + Debug + Default + Eq + UniformRand + ToBytes + FromBytes;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError>;

    fn parameters(&self) -> &Self::Parameters;
}
