use snarkos_errors::algorithms::CRHError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::{fmt::Debug, hash::Hash, path::PathBuf};

pub trait CRH: From<<Self as CRH>::Parameters> + Clone {
    type Output: Debug + ToBytes + FromBytes + Clone + Eq + Hash + Default;
    type Parameters: Clone + ToBytes + FromBytes;

    const INPUT_SIZE_BITS: usize;

    fn setup<R: Rng>(r: &mut R) -> Self;

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError>;

    fn parameters(&self) -> &Self::Parameters;

    fn store(&self, path: &PathBuf) -> Result<(), CRHError>;

    fn load(path: &PathBuf) -> Result<Self, CRHError>;
}
