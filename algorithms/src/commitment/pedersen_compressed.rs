use crate::{
    commitment::{PedersenCommitment, PedersenCommitmentParameters},
    crh::PedersenSize,
};
use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::{
    algorithms::CommitmentScheme,
    curves::{AffineCurve, Group, ProjectiveCurve},
};

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCompressedCommitment<G: Group + ProjectiveCurve, S: PedersenSize> {
    pub parameters: PedersenCommitmentParameters<G, S>,
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> CommitmentScheme for PedersenCompressedCommitment<G, S> {
    type Output = <G::Affine as AffineCurve>::BaseField;
    type Parameters = PedersenCommitmentParameters<G, S>;
    type Randomness = <G as Group>::ScalarField;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: PedersenCommitmentParameters::new(rng),
        }
    }

    /// Returns the affine x-coordinate as the commitment.
    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError> {
        let commitment = PedersenCommitment::<G, S> {
            parameters: self.parameters.clone(),
        };

        let output = commitment.commit(input, randomness)?;
        let affine = output.into_affine();
        debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
        Ok(affine.to_x_coordinate())
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> PedersenCompressedCommitment<G, S> {
    /// Returns the affine as the commitment.
    pub fn commit_to_affine(&self, input: &[u8], randomness: &<G as Group>::ScalarField) -> Result<G::Affine, CommitmentError> {
        let commitment = PedersenCommitment::<G, S> {
            parameters: self.parameters.clone(),
        };

        let output = commitment.commit(input, randomness)?;
        let affine = output.into_affine();
        debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
        Ok(affine)
    }
}