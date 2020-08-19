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
            parameters: PedersenCommitmentParameters::setup(rng),
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

impl<G: Group + ProjectiveCurve, S: PedersenSize> From<PedersenCommitmentParameters<G, S>>
    for PedersenCompressedCommitment<G, S>
{
    fn from(parameters: PedersenCommitmentParameters<G, S>) -> Self {
        Self { parameters }
    }
}
