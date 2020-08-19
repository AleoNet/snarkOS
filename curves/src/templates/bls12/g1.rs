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

use crate::templates::{
    bls12::Bls12Parameters,
    short_weierstrass::short_weierstrass_jacobian::{GroupAffine, GroupProjective},
};
use snarkos_errors::serialization::SerializationError;
use snarkos_models::curves::{pairing_engine::AffineCurve, Zero};
use snarkos_utilities::{bytes::ToBytes, serialize::*};

use std::io::{Result as IoResult, Write};

pub type G1Affine<P> = GroupAffine<<P as Bls12Parameters>::G1Parameters>;
pub type G1Projective<P> = GroupProjective<<P as Bls12Parameters>::G1Parameters>;

#[derive(Derivative, CanonicalSerialize, CanonicalDeserialize)]
#[derivative(
    Clone(bound = "P: Bls12Parameters"),
    Debug(bound = "P: Bls12Parameters"),
    PartialEq(bound = "P: Bls12Parameters"),
    Eq(bound = "P: Bls12Parameters")
)]
pub struct G1Prepared<P: Bls12Parameters>(pub G1Affine<P>);

impl<P: Bls12Parameters> G1Prepared<P> {
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn from_affine(p: G1Affine<P>) -> Self {
        G1Prepared(p)
    }
}

impl<P: Bls12Parameters> Default for G1Prepared<P> {
    fn default() -> Self {
        G1Prepared(G1Affine::<P>::prime_subgroup_generator())
    }
}

impl<P: Bls12Parameters> ToBytes for G1Prepared<P> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.0.write(writer)
    }
}
