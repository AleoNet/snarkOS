use crate::{commitment::PedersenCommitmentParameters, crh::PedersenSize};
use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::{Group, PrimeField},
};
use snarkos_utilities::bititerator::BitIterator;

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCommitment<G: Group, S: PedersenSize> {
    pub parameters: PedersenCommitmentParameters<G, S>,
}

impl<G: Group, S: PedersenSize> CommitmentScheme for PedersenCommitment<G, S> {
    type Output = G;
    type Parameters = PedersenCommitmentParameters<G, S>;
    type Randomness = G::ScalarField;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: PedersenCommitmentParameters::setup(rng),
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
}

impl<G: Group, S: PedersenSize> From<PedersenCommitmentParameters<G, S>> for PedersenCommitment<G, S> {
    fn from(parameters: PedersenCommitmentParameters<G, S>) -> Self {
        Self { parameters }
    }
}
