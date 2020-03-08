use snarkos_errors::algorithms::PRFError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::{fmt::Debug, hash::Hash};

pub trait PRF {
    type Input: FromBytes + Default;
    type Output: ToBytes + Eq + Clone + Default + Hash;
    type Seed: FromBytes + ToBytes + Clone + Default + Debug;

    fn evaluate(seed: &Self::Seed, input: &Self::Input) -> Result<Self::Output, PRFError>;
}
