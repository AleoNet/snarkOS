use crate::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::{algorithms::CRHError, curves::ConstraintFieldError};
use snarkos_models::{
    algorithms::CRH,
    curves::{to_field_vec::ToConstraintField, AffineCurve, Field, Group, ProjectiveCurve},
};

use rand::Rng;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCompressedCRH<G: Group + ProjectiveCurve, S: PedersenSize> {
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

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }

    /// Store the Pedersen compressed CRH parameters to a file at the given path.
    fn store(&self, path: &PathBuf) -> Result<(), CRHError> {
        self.parameters.store(path)?;

        Ok(())
    }

    /// Load the Pedersen Compressed CRH parameters from a file at the given path.
    fn load(path: &PathBuf) -> Result<Self, CRHError> {
        let parameters = PedersenCRHParameters::<G, S>::load(path)?;

        Ok(Self { parameters })
    }
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> From<PedersenCRHParameters<G, S>> for PedersenCompressedCRH<G, S> {
    fn from(parameters: PedersenCRHParameters<G, S>) -> Self {
        Self { parameters }
    }
}

impl<F: Field, G: Group + ProjectiveCurve + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F>
    for PedersenCompressedCRH<G, S>
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.parameters.to_field_elements()
    }
}
