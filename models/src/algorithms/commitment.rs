use snarkos_errors::algorithms::Error;
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait CommitmentScheme: Sized {
    type Parameters: Clone;
    type Randomness: Clone + ToBytes + Default + Eq + UniformRand + Debug;
    type Output: ToBytes + Clone + Default + Eq + Hash + Debug;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, Error>;
}
