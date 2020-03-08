use crate::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::algorithms::CRHError;
use snarkos_models::{
    algorithms::CRH,
    curves::{AffineCurve, Group, ProjectiveCurve},
};

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCompressedCRH<G: Group, S: PedersenSize> {
    pub parameters: PedersenCRHParameters<G, S>,
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> CRH for PedersenCompressedCRH<G, S> {
    type Output = <G::Affine as AffineCurve>::BaseField;
    type Parameters = PedersenCRHParameters<G, S>;

    const INPUT_SIZE_BITS: usize = S::WINDOW_SIZE * S::NUM_WINDOWS;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self {
            parameters: PedersenCRHParameters::new(rng),
        }
    }

    /// Returns the affine x-coordinate as the collision-resistant hash output.
    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError> {
        let crh = PedersenCRH::<G, S> {
            parameters: self.parameters.clone(),
        };

        let output = crh.hash(input)?;
        let affine = output.into_affine();
        debug_assert!(affine.is_in_correct_subgroup_assuming_on_curve());
        Ok(affine.to_x_coordinate())
    }
}

impl<G: Group, S: PedersenSize> From<PedersenCRHParameters<G, S>> for PedersenCompressedCRH<G, S> {
    fn from(parameters: PedersenCRHParameters<G, S>) -> Self {
        Self { parameters }
    }
}
