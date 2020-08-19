// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::{algorithms::CRHError, curves::ConstraintFieldError};
use snarkos_models::{
    algorithms::{CRHParameters, CRH},
    curves::{to_field_vec::ToConstraintField, AffineCurve, Field, Group, ProjectiveCurve},
};

use rand::Rng;

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
            parameters: PedersenCRHParameters::setup(rng),
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
