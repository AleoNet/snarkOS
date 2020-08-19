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
    algorithms::CommitmentScheme,
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            eq::{ConditionalEqGadget, EqGadget},
            select::CondSelectGadget,
            uint::UInt8,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait CommitmentGadget<C: CommitmentScheme, F: Field> {
    type OutputGadget: ConditionalEqGadget<F>
        + CondSelectGadget<F>
        + EqGadget<F>
        + ToBytesGadget<F>
        + AllocGadget<C::Output, F>
        + Clone
        + Sized
        + Debug;
    type ParametersGadget: AllocGadget<C::Parameters, F> + Clone;
    type RandomnessGadget: AllocGadget<C::Randomness, F> + Clone;

    fn check_commitment_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        r: &Self::RandomnessGadget,
    ) -> Result<Self::OutputGadget, SynthesisError>;
}
