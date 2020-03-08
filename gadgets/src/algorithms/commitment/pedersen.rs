use snarkos_algorithms::{
    commitment::{PedersenCommitment, PedersenCommitmentParameters},
    crh::PedersenSize,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField},
    gadgets::{
        algorithms::CommitmentGadget,
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, uint8::UInt8},
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
pub struct PedersenRandomnessGadget<G: Group>(Vec<UInt8>, PhantomData<G>);

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
        // Pad if input length is less than `W::WINDOW_SIZE * W::NUM_WINDOWS`.
        if (input.len() * 8) < S::WINDOW_SIZE * S::NUM_WINDOWS {
            let current_length = input.len();
            for _ in current_length..((S::WINDOW_SIZE * S::NUM_WINDOWS) / 8) {
                padded_input.push(UInt8::constant(0u8));
            }
        }

        assert_eq!(padded_input.len() * 8, S::WINDOW_SIZE * S::NUM_WINDOWS);
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);

        // Allocate new variable for commitment output.
        let input_in_bits: Vec<_> = padded_input.iter().flat_map(|byte| byte.into_bits_le()).collect();
        let input_in_bits = input_in_bits.chunks(S::WINDOW_SIZE);
        let mut result =
            GG::precomputed_base_multiscalar_mul(cs.ns(|| "msm"), &parameters.parameters.bases, input_in_bits)?;

        // Compute h^r
        let rand_bits: Vec<_> = randomness.0.iter().flat_map(|byte| byte.into_bits_le()).collect();
        result.precomputed_base_scalar_mul(
            cs.ns(|| "randomizer"),
            rand_bits.iter().zip(&parameters.parameters.random_base),
        )?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::curves::edwards_bls12::EdwardsBlsGadget;
    use snarkos_algorithms::{commitment::PedersenCommitment, crh::PedersenSize};
    use snarkos_curves::edwards_bls12::{EdwardsProjective, Fq, Fr};
    use snarkos_models::{
        algorithms::CommitmentScheme,
        curves::ProjectiveCurve,
        gadgets::{
            algorithms::CommitmentGadget,
            r1cs::{ConstraintSystem, TestConstraintSystem},
            utilities::uint8::UInt8,
        },
    };
    use snarkos_utilities::rand::UniformRand;

    use rand::thread_rng;

    #[test]
    fn commitment_gadget_test() {
        let mut cs = TestConstraintSystem::<Fq>::new();

        #[derive(Clone, PartialEq, Eq, Hash)]
        pub(super) struct Size;

        impl PedersenSize for Size {
            const NUM_WINDOWS: usize = 8;
            const WINDOW_SIZE: usize = 4;
        }

        type TestCommitment = PedersenCommitment<EdwardsProjective, Size>;
        type TestCommitmentGadget = PedersenCommitmentGadget<EdwardsProjective, Fq, EdwardsBlsGadget>;

        let rng = &mut thread_rng();

        let input = [1u8; 4];
        let randomness = Fr::rand(rng);
        let commitment = PedersenCommitment::<EdwardsProjective, Size>::setup(rng);
        let native_output = commitment.commit(&input, &randomness).unwrap();

        let mut input_bytes = vec![];
        for (byte_i, input_byte) in input.iter().enumerate() {
            let cs = cs.ns(|| format!("input_byte_gadget_{}", byte_i));
            input_bytes.push(UInt8::alloc(cs, || Ok(*input_byte)).unwrap());
        }

        let randomness_gadget =
            <TestCommitmentGadget as CommitmentGadget<TestCommitment, Fq>>::RandomnessGadget::alloc(
                &mut cs.ns(|| "randomness_gadget"),
                || Ok(&randomness),
            )
            .unwrap();
        let parameters_gadget =
            <TestCommitmentGadget as CommitmentGadget<TestCommitment, Fq>>::ParametersGadget::alloc(
                &mut cs.ns(|| "parameters_gadget"),
                || Ok(&commitment.parameters),
            )
            .unwrap();
        let output_gadget = <TestCommitmentGadget as CommitmentGadget<TestCommitment, Fq>>::check_commitment_gadget(
            &mut cs.ns(|| "commitment_gadget"),
            &parameters_gadget,
            &input_bytes,
            &randomness_gadget,
        )
        .unwrap();

        let native_output = native_output.into_affine();
        assert_eq!(native_output.x, output_gadget.x.value.unwrap());
        assert_eq!(native_output.y, output_gadget.y.value.unwrap());
        assert!(cs.is_satisfied());
    }
}
