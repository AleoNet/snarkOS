use snarkos_errors::algorithms::CommitmentError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait CommitmentScheme: Sized + Clone {
    type Output: Clone + Default + Eq + Hash + Debug + ToBytes + FromBytes;
    type Parameters: Clone + ToBytes + FromBytes;
    type Randomness: Clone + Default + Eq + UniformRand + Debug + ToBytes + FromBytes;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError>;

    fn parameters(&self) -> &Self::Parameters;
}
