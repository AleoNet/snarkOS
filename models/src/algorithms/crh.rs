use snarkos_errors::algorithms::CRHError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

pub trait CRHParameters: Clone + Debug + ToBytes + FromBytes + Eq {
    fn setup<R: Rng>(r: &mut R) -> Self;
}

pub trait CRH: Clone + From<<Self as CRH>::Parameters> {
    type Output: Clone + Debug + Display + ToBytes + FromBytes + Eq + Hash + Default;
    type Parameters: CRHParameters;

    const INPUT_SIZE_BITS: usize;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError>;

    fn parameters(&self) -> &Self::Parameters;
}
