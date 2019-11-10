use crate::{commitment::PedersenCommitmentParameters, crh::PedersenSize};
use snarkos_errors::algorithms::{CryptoError, Error};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::{AffineCurve, Group, PrimeField, ProjectiveCurve},
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
            parameters: PedersenCommitmentParameters::new(rng),
        }
    }

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, Error> {
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
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> PedersenCommitment<G, S> {
    /// Returns the affine x-coordinate of a given commitment.
    fn compress(output: G) -> Result<<G::Affine as AffineCurve>::BaseField, CryptoError> {
        let affine = output.into_affine();
        debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
        Ok(affine.to_x_coordinate())
    }
}
