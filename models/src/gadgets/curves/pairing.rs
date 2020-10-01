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
    curves::{Field, PairingEngine},
    gadgets::{
        curves::{FieldGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::ToBytesGadget,
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait PairingGadget<Pairing: PairingEngine, F: Field> {
    type G1Gadget: GroupGadget<Pairing::G1Projective, F>;
    type G2Gadget: GroupGadget<Pairing::G2Projective, F>;
    type G1PreparedGadget: ToBytesGadget<F> + Clone + Debug;
    type G2PreparedGadget: ToBytesGadget<F> + Clone + Debug;
    type GTGadget: FieldGadget<Pairing::Fqk, F> + Clone;

    fn miller_loop<CS: ConstraintSystem<F>>(
        cs: CS,
        p: &[Self::G1PreparedGadget],
        q: &[Self::G2PreparedGadget],
    ) -> Result<Self::GTGadget, SynthesisError>;

    fn final_exponentiation<CS: ConstraintSystem<F>>(
        cs: CS,
        p: &Self::GTGadget,
    ) -> Result<Self::GTGadget, SynthesisError>;

    fn pairing<CS: ConstraintSystem<F>>(
        mut cs: CS,
        p: Self::G1PreparedGadget,
        q: Self::G2PreparedGadget,
    ) -> Result<Self::GTGadget, SynthesisError> {
        let tmp = Self::miller_loop(cs.ns(|| "miller loop"), &[p], &[q])?;
        Self::final_exponentiation(cs.ns(|| "final_exp"), &tmp)
    }

    /// Computes a product of pairings.
    fn product_of_pairings<CS: ConstraintSystem<F>>(
        mut cs: CS,
        p: &[Self::G1PreparedGadget],
        q: &[Self::G2PreparedGadget],
    ) -> Result<Self::GTGadget, SynthesisError> {
        let miller_result = Self::miller_loop(&mut cs.ns(|| "Miller loop"), p, q)?;
        Self::final_exponentiation(&mut cs.ns(|| "Final Exp"), &miller_result)
    }

    fn prepare_g1<CS: ConstraintSystem<F>>(
        cs: CS,
        q: &Self::G1Gadget,
    ) -> Result<Self::G1PreparedGadget, SynthesisError>;

    fn prepare_g2<CS: ConstraintSystem<F>>(
        cs: CS,
        q: &Self::G2Gadget,
    ) -> Result<Self::G2PreparedGadget, SynthesisError>;
}
