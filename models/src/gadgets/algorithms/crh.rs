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
    algorithms::CRH,
    curves::{Field, PrimeField},
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            eq::{ConditionalEqGadget, EqGadget},
            select::CondSelectGadget,
            uint::{UInt, UInt8},
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait CRHGadget<H: CRH, F: Field>: Sized + Clone {
    type ParametersGadget: AllocGadget<H::Parameters, F> + Clone;
    type OutputGadget: ConditionalEqGadget<F>
        + EqGadget<F>
        + ToBytesGadget<F>
        + CondSelectGadget<F>
        + AllocGadget<H::Output, F>
        + Debug
        + Clone
        + Sized;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}

pub trait MaskedCRHGadget<H: CRH, F: PrimeField>: CRHGadget<H, F> {
    /// Extends the mask such that 0 => 01, 1 => 10.
    fn extend_mask<CS: ConstraintSystem<F>>(_: CS, mask: &[UInt8]) -> Result<Vec<UInt8>, SynthesisError> {
        let extended_mask = mask
            .iter()
            .flat_map(|m| {
                m.to_bits_le()
                    .chunks(4)
                    .map(|c| {
                        let new_byte = c.into_iter().flat_map(|b| vec![*b, b.not()]).collect::<Vec<_>>();
                        UInt8::from_bits_le(&new_byte)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(extended_mask)
    }

    fn check_evaluation_gadget_masked<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        mask_parameters: &Self::ParametersGadget,
        mask: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}
