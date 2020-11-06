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

use crate::algorithms::crh::pedersen::PedersenCRHParametersGadget;
use snarkos_algorithms::crh::{
    BoweHopwoodPedersenCRH,
    BoweHopwoodPedersenCompressedCRH,
    PedersenSize,
    BOWE_HOPWOOD_CHUNK_SIZE,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, ProjectiveCurve},
    gadgets::{
        algorithms::CRHGadget,
        curves::{CompressedGroupGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::{
            boolean::Boolean,
            uint::unsigned_integer::{UInt, UInt8},
        },
    },
};

use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoweHopwoodPedersenCRHGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<F>,
}

impl<F: Field, G: Group, GG: GroupGadget<G, F>, S: PedersenSize> CRHGadget<BoweHopwoodPedersenCRH<G, S>, F>
    for BoweHopwoodPedersenCRHGadget<G, F, GG>
{
    type OutputGadget = GG;
    type ParametersGadget = PedersenCRHParametersGadget<G, S, F, GG>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        // Pad the input bytes
        let mut padded_input_bytes = input.to_vec();
        padded_input_bytes.resize(S::WINDOW_SIZE * S::NUM_WINDOWS / 8, UInt8::constant(0u8));
        assert_eq!(padded_input_bytes.len() * 8, S::WINDOW_SIZE * S::NUM_WINDOWS);

        // Pad the input bits if it is not the current length.
        let mut input_in_bits: Vec<_> = padded_input_bytes
            .into_iter()
            .flat_map(|byte| byte.to_bits_le())
            .collect();
        if (input_in_bits.len()) % BOWE_HOPWOOD_CHUNK_SIZE != 0 {
            let current_length = input_in_bits.len();
            let target_length = current_length + BOWE_HOPWOOD_CHUNK_SIZE - current_length % BOWE_HOPWOOD_CHUNK_SIZE;
            input_in_bits.resize(target_length, Boolean::constant(false));
        }
        assert!(input_in_bits.len() % BOWE_HOPWOOD_CHUNK_SIZE == 0);
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);
        for generators in parameters.parameters.bases.iter() {
            assert_eq!(generators.len(), S::WINDOW_SIZE);
        }

        // Allocate new variable for the result.
        let input_in_bits = input_in_bits
            .chunks(S::WINDOW_SIZE * BOWE_HOPWOOD_CHUNK_SIZE)
            .map(|x| x.chunks(BOWE_HOPWOOD_CHUNK_SIZE).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let result =
            GG::precomputed_base_3_bit_signed_digit_scalar_mul(cs, &parameters.parameters.bases, &input_in_bits)?;

        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoweHopwoodPedersenCompressedCRHGadget<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>>
{
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<F>,
}

impl<F: Field, G: Group + ProjectiveCurve, GG: CompressedGroupGadget<G, F>, S: PedersenSize>
    CRHGadget<BoweHopwoodPedersenCompressedCRH<G, S>, F> for BoweHopwoodPedersenCompressedCRHGadget<G, F, GG>
{
    type OutputGadget = GG::BaseFieldGadget;
    type ParametersGadget = PedersenCRHParametersGadget<G, S, F, GG>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let output = BoweHopwoodPedersenCRHGadget::<G, F, GG>::check_evaluation_gadget(cs, parameters, input)?;
        Ok(output.to_x_coordinate())
    }
}
