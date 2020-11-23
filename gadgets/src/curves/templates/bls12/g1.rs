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

use crate::curves::templates::bls12::AffineGadget;
use snarkos_curves::templates::bls12::{Bls12Parameters, G1Prepared};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::ProjectiveCurve,
    gadgets::{
        curves::{FpGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::{uint::UInt8, ToBytesGadget},
    },
};

use std::fmt::Debug;

pub type G1Gadget<P> = AffineGadget<
    <P as Bls12Parameters>::G1Parameters,
    <P as Bls12Parameters>::Fp,
    FpGadget<<P as Bls12Parameters>::Fp>,
>;

#[derive(Derivative)]
#[derivative(Clone(bound = "G1Gadget<P>: Clone"), Debug(bound = "G1Gadget<P>: Debug"))]
pub struct G1PreparedGadget<P: Bls12Parameters>(pub G1Gadget<P>);

impl<P: Bls12Parameters> G1PreparedGadget<P> {
    pub fn get_value(&self) -> Option<G1Prepared<P>> {
        Some(G1Prepared::from_affine(self.0.get_value().unwrap().into_affine()))
    }

    pub fn from_affine<CS: ConstraintSystem<P::Fp>>(_cs: CS, q: G1Gadget<P>) -> Result<Self, SynthesisError> {
        Ok(G1PreparedGadget(q))
    }
}

impl<P: Bls12Parameters> ToBytesGadget<P::Fp> for G1PreparedGadget<P> {
    #[inline]
    fn to_bytes<CS: ConstraintSystem<P::Fp>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.0.to_bytes(&mut cs.ns(|| "g_alpha to bytes"))
    }

    fn to_bytes_strict<CS: ConstraintSystem<P::Fp>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}
