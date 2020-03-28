use crate::{commitment::PedersenCommitmentParameters, crh::PedersenSize};
use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::{Group, PrimeField},
};
use snarkos_utilities::{
    bititerator::BitIterator,
    bytes::{FromBytes, ToBytes},
};

use rand::Rng;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCommitment<G: Group, S: PedersenSize> {
    pub parameters: PedersenCommitmentParameters<G, S>,
}

impl<G: Group, S: PedersenSize> ToBytes for PedersenCommitment<G, S> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.parameters.write(&mut writer)
    }
}

impl<G: Group, S: PedersenSize> FromBytes for PedersenCommitment<G, S> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let parameters: PedersenCommitmentParameters<G, S> = FromBytes::read(&mut reader)?;
        Ok(Self { parameters })
    }
}

impl<G: Group, S: PedersenSize> CommitmentScheme for PedersenCommitment<G, S> {
    type Output = G;
    type Parameters = PedersenCommitmentParameters<G, S>;
    type Randomness = G::ScalarField;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: PedersenCommitmentParameters::new(rng),
        }
    }

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError> {
        // If the input is too long, return an error.
        if input.len() > S::WINDOW_SIZE * S::NUM_WINDOWS {
            // TODO (howardwu): Return a CommitmentError.
            panic!("incorrect input length: {:?}", input.len());
        }

        let mut output = self.parameters.crh.hash(&input)?;

        // Compute h^r.
        let mut scalar_bits = BitIterator::new(randomness.into_repr()).collect::<Vec<_>>();
        scalar_bits.reverse();
        for (bit, power) in scalar_bits.into_iter().zip(&self.parameters.random_base) {
            if bit {
                output += power
            }
        }

        Ok(output)
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }

    /// Store the Pedersen commitment parameters to a file at the given path.
    fn store(&self, path: &PathBuf) -> Result<(), CommitmentError> {
        self.parameters.store(path)?;
        Ok(())
    }

    /// Load the Pedersen commitment parameters from a file at the given path.
    fn load(path: &PathBuf) -> Result<Self, CommitmentError> {
        let parameters = PedersenCommitmentParameters::<G, S>::load(path)?;

        Ok(Self { parameters })
    }
}
