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

use snarkos_algorithms::{
    commitment::{PedersenCommitment, PedersenCommitmentParameters, PedersenCompressedCommitment},
    crh::PedersenSize,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField, ProjectiveCurve},
    gadgets::{
        algorithms::CommitmentGadget,
        curves::{CompressedGroupGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            uint::unsigned_integer::{UInt, UInt8},
        },
    },
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone)]
pub struct PedersenCommitmentParametersGadget<G: Group, S: PedersenSize, F: Field> {
    parameters: PedersenCommitmentParameters<G, S>,
    _group: PhantomData<G>,
    _engine: PhantomData<F>,
    _window: PhantomData<S>,
}

impl<G: Group, S: PedersenSize, F: PrimeField> AllocGadget<PedersenCommitmentParameters<G, S>, F>
    for PedersenCommitmentParametersGadget<G, S, F>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<PedersenCommitmentParameters<G, S>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let temp = value_gen()?;
        let parameters = temp.borrow().clone();
        Ok(PedersenCommitmentParametersGadget {
            parameters,
            _group: PhantomData,
            _engine: PhantomData,
            _window: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<PedersenCommitmentParameters<G, S>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let temp = value_gen()?;
        let parameters = temp.borrow().clone();
        Ok(PedersenCommitmentParametersGadget {
            parameters,
            _group: PhantomData,
            _engine: PhantomData,
            _window: PhantomData,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PedersenRandomnessGadget<G: Group>(pub Vec<UInt8>, PhantomData<G>);

impl<G: Group, F: PrimeField> AllocGadget<G::ScalarField, F> for PedersenRandomnessGadget<G> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G::ScalarField>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let randomness = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(PedersenRandomnessGadget(
            UInt8::alloc_vec(cs, &randomness)?,
            PhantomData,
        ))
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G::ScalarField>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let randomness = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(PedersenRandomnessGadget(
            UInt8::alloc_input_vec(cs, &randomness)?,
            PhantomData,
        ))
    }
}

pub struct PedersenCommitmentGadget<G: Group, F: Field, GG: GroupGadget<G, F>>(
    PhantomData<G>,
    PhantomData<GG>,
    PhantomData<F>,
);

impl<F: PrimeField, G: Group, GG: GroupGadget<G, F>, S: PedersenSize> CommitmentGadget<PedersenCommitment<G, S>, F>
    for PedersenCommitmentGadget<G, F, GG>
{
    type OutputGadget = GG;
    type ParametersGadget = PedersenCommitmentParametersGadget<G, S, F>;
    type RandomnessGadget = PedersenRandomnessGadget<G>;

    fn check_commitment_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        randomness: &Self::RandomnessGadget,
    ) -> Result<Self::OutputGadget, SynthesisError> {
        assert!((input.len() * 8) <= (S::WINDOW_SIZE * S::NUM_WINDOWS));

        let mut padded_input = input.to_vec();
        // Pad if input length is less than `S::WINDOW_SIZE * S::NUM_WINDOWS`.
        if (input.len() * 8) < S::WINDOW_SIZE * S::NUM_WINDOWS {
            let current_length = input.len();
            for _ in current_length..((S::WINDOW_SIZE * S::NUM_WINDOWS) / 8) {
                padded_input.push(UInt8::constant(0u8));
            }
        }

        assert_eq!(padded_input.len() * 8, S::WINDOW_SIZE * S::NUM_WINDOWS);
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);

        // Allocate new variable for commitment output.
        let input_in_bits: Vec<_> = padded_input.iter().flat_map(|byte| byte.to_bits_le()).collect();
        let input_in_bits = input_in_bits.chunks(S::WINDOW_SIZE);
        let mut result =
            GG::precomputed_base_multiscalar_mul(cs.ns(|| "msm"), &parameters.parameters.bases, input_in_bits)?;

        // Compute h^r
        let rand_bits: Vec<_> = randomness.0.iter().flat_map(|byte| byte.to_bits_le()).collect();
        result.precomputed_base_scalar_mul(
            cs.ns(|| "randomizer"),
            rand_bits.iter().zip(&parameters.parameters.random_base),
        )?;

        Ok(result)
    }
}

pub struct PedersenCompressedCommitmentGadget<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>>(
    PhantomData<G>,
    PhantomData<GG>,
    PhantomData<F>,
);

impl<F: PrimeField, G: Group + ProjectiveCurve, GG: CompressedGroupGadget<G, F>, S: PedersenSize>
    CommitmentGadget<PedersenCompressedCommitment<G, S>, F> for PedersenCompressedCommitmentGadget<G, F, GG>
{
    type OutputGadget = GG::BaseFieldGadget;
    type ParametersGadget = PedersenCommitmentParametersGadget<G, S, F>;
    type RandomnessGadget = PedersenRandomnessGadget<G>;

    fn check_commitment_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        randomness: &Self::RandomnessGadget,
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let output = PedersenCommitmentGadget::<G, F, GG>::check_commitment_gadget(cs, parameters, input, randomness)?;
        Ok(output.to_x_coordinate())
    }
}
