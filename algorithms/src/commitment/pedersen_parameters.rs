use crate::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::curves::{to_field_vec::ToConstraintField, Field, Group};

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCommitmentParameters<G: Group, S: PedersenSize> {
    pub bases: Vec<Vec<G>>,
    pub random_base: Vec<G>,
    pub crh: PedersenCRH<G, S>,
}

impl<G: Group, S: PedersenSize> PedersenCommitmentParameters<G, S> {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        let bases = (0..S::NUM_WINDOWS)
            .map(|_| Self::base(S::WINDOW_SIZE, rng))
            .collect::<Vec<Vec<G>>>();
        let random_base = Self::base(S::WINDOW_SIZE, rng);
        let crh_parameters = PedersenCRHParameters::from(bases.clone());
        let crh = PedersenCRH::from(crh_parameters);
        Self {
            bases,
            random_base,
            crh,
        }
    }

    fn base<R: Rng>(num_powers: usize, rng: &mut R) -> Vec<G> {
        let mut powers = vec![];
        let mut base = G::rand(rng);
        for _ in 0..num_powers {
            powers.push(base);
            base.double_in_place();
        }
        powers
    }
}

impl<F: Field, G: Group + ToConstraintField<F>, S: PedersenSize> ToConstraintField<F>
    for PedersenCommitmentParameters<G, S>
{
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(Vec::new())
    }
}
