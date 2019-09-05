use snarkos_errors::algorithms::PRFError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::{fmt::Debug, hash::Hash};

pub trait PRF {
    type Seed: FromBytes + ToBytes + Clone + Default + Debug;
    type Input: FromBytes + Default;
    type Output: ToBytes + Eq + Clone + Default + Hash;

    fn evaluate(seed: &Self::Seed, input: &Self::Input) -> Result<Self::Output, PRFError>;
}
