use snarkos_errors::algorithms::CRHError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::{fmt::Debug, hash::Hash};

pub trait CRH: From<<Self as CRH>::Parameters> + Clone {
    type Output: Debug + ToBytes + FromBytes + Clone + Eq + Hash + Default;
    type Parameters: Clone;

    const INPUT_SIZE_BITS: usize;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError>;

    fn parameters(&self) -> &Self::Parameters;
}
