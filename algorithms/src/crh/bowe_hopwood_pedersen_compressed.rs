use crate::crh::{BoweHopwoodPedersenCRH, PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::{algorithms::CRHError, curves::ConstraintFieldError};
use snarkos_models::{
    algorithms::CRH,
    curves::{to_field_vec::ToConstraintField, AffineCurve, Field, Group, ProjectiveCurve},
};

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoweHopwoodPedersenCompressedCRH<G: Group + ProjectiveCurve, S: PedersenSize> {
    pub parameters: PedersenCRHParameters<G, S>,
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> CRH for BoweHopwoodPedersenCompressedCRH<G, S> {
    type Output = <G::Affine as AffineCurve>::BaseField;
    type Parameters = PedersenCRHParameters<G, S>;

    const INPUT_SIZE_BITS: usize = PedersenCRH::<G, S>::INPUT_SIZE_BITS;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        let parameters = BoweHopwoodPedersenCRH::<G, S>::setup(rng).parameters;

        Self { parameters }
    }

    fn hash(&self, input: &[u8]) -> Result<Self::Output, CRHError> {
        let crh = BoweHopwoodPedersenCRH::<G, S> {
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
}

impl<G: Group + ProjectiveCurve, S: PedersenSize> From<PedersenCRHParameters<G, S>>
    for BoweHopwoodPedersenCompressedCRH<G, S>
{
    fn from(parameters: PedersenCRHParameters<G, S>) -> Self {
        Self { parameters }
    }
}

impl<F: Field, G: Group + ProjectiveCurve + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F>
    for BoweHopwoodPedersenCompressedCRH<G, S>
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.parameters.to_field_elements()
    }
}
